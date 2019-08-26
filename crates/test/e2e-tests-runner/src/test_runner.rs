//! Implementation for run tests in browser, check and print results.

use std::{
    fs::File,
    io::{prelude::*, Error as IoError},
    path::{Path, PathBuf},
};

use clap::ArgMatches;
use failure::Fail;
use fantoccini::{
    error::{CmdError, NewSessionError},
    Client, Locator,
};
use futures::{
    future::{Either, Loop},
    Future,
};
use serde_json::json;
use webdriver::capabilities::Capabilities;

use crate::mocha_result::TestResults;

/// Errors which can occur in [`TestRunner`].
#[allow(clippy::pub_enum_variant_names)]
#[derive(Debug, Fail)]
pub enum Error {
    /// WebDriver command failed.
    #[fail(display = "WebDriver command failed: {:?}", _0)]
    CmdErr(CmdError),

    /// WebDriver startup failed.
    #[fail(display = "WebDriver startup failed: {:?}", _0)]
    NewSessionError(NewSessionError),

    /// Test results not found in browser logs.
    #[fail(display = "Test results not found in browser logs. Probably \
                      something wrong with template. See printed browser \
                      logs for more info.")]
    TestResultsNotFoundInLogs,

    /// Some test failed.
    #[fail(display = "Some test failed.")]
    TestsFailed,
}

impl From<CmdError> for Error {
    fn from(err: CmdError) -> Self {
        Error::CmdErr(err)
    }
}

impl From<NewSessionError> for Error {
    fn from(err: NewSessionError) -> Self {
        Error::NewSessionError(err)
    }
}

/// Delete all generated tests html from test dir.
fn delete_all_tests_htmls(path_test_dir: &Path) -> Result<(), IoError> {
    for entry in std::fs::read_dir(path_test_dir)? {
        let entry = entry?;
        let path = entry.path();
        if let Some(ext) = path.extension() {
            if ext == "html" {
                std::fs::remove_file(path)?;
            }
        }
    }
    Ok(())
}

/// Medea's e2e tests runner.
///
/// Run e2e tests in browser, check results, print results.
pub struct TestRunner {
    tests: Vec<PathBuf>,
    test_addr: String,
}

impl TestRunner {
    /// Run e2e tests.
    pub fn run(
        path_to_tests: PathBuf,
        opts: &ArgMatches,
    ) -> impl Future<Item = (), Error = Error> {
        let test_addr = opts.value_of("tests_files_addr").unwrap().to_string();
        if path_to_tests.is_dir() {
            let tests = get_all_tests_paths(&path_to_tests);
            let runner = Self { test_addr, tests };
            Either::A(runner.run_tests(&opts).then(move |err| {
                delete_all_tests_htmls(&path_to_tests).unwrap();
                err
            }))
        } else {
            let runner = Self {
                test_addr,
                tests: vec![path_to_tests.clone()],
            };
            Either::B(runner.run_tests(&opts).then(move |err| {
                let test_dir = path_to_tests.parent().unwrap();
                delete_all_tests_htmls(&test_dir).unwrap();
                err
            }))
        }
    }

    /// Create WebDriver client, start e2e tests loop.
    fn run_tests(
        self,
        opts: &ArgMatches,
    ) -> impl Future<Item = (), Error = Error> {
        let caps = get_webdriver_capabilities(opts);
        Client::with_capabilities(
            opts.value_of("webdriver_addr").unwrap(),
            caps,
        )
        .map_err(Error::from)
        .and_then(|client| self.tests_loop(client))
        .map_err(Error::from)
    }

    /// Tests loop which alternately launches tests in browser.
    ///
    /// This future resolve when all tests completed or when test failed.
    ///
    /// Returns [`Error::TestsFailed`] if some test failed.
    fn tests_loop(
        self,
        client: Client,
    ) -> impl Future<Item = (), Error = Error> {
        futures::future::loop_fn((client, self), |(client, mut runner)| {
            if let Some(test) = runner.tests.pop() {
                let test_path = generate_and_save_test_html(&test);
                let test_url = runner.get_url_to_test(&test_path);
                println!(
                    "\nRunning {} test...",
                    test.file_name().unwrap().to_str().unwrap()
                );
                Either::A(
                    client
                        .goto(&test_url)
                        .and_then(wait_for_test_end)
                        .map_err(Error::from)
                        .and_then(|client| runner.check_test_results(client))
                        .map_err(Error::from)
                        .map(Loop::Continue),
                )
            } else {
                Either::B(futures::future::ok(Loop::Break(())))
            }
        })
        .map_err(Error::from)
    }

    /// Check results of tests.
    ///
    /// This function will close WebDriver's session if some error happen.
    ///
    /// Returns [`Error::TestsFailed`] if some test failed.
    ///
    /// Returns [`Error::TestResultsNotFoundInLogs`] if mocha results not found
    /// in JS side console logs.
    fn check_test_results(
        self,
        mut client: Client,
    ) -> impl Future<Item = (Client, Self), Error = Error> {
        client
            .execute("return console.logs", Vec::new())
            .map_err(|e| panic!("{:?}", e))
            .map(move |e| (e, client))
            .and_then(move |(result, client)| {
                let logs = result.as_array().unwrap();
                for message in logs {
                    let message =
                        message.as_array().unwrap()[0].as_str().unwrap();
                    if let Ok(test_results) =
                        serde_json::from_str::<TestResults>(message)
                    {
                        println!("{}", test_results);
                        if test_results.is_has_error() {
                            return Err((client, Error::TestsFailed));
                        } else {
                            return Ok((client, self));
                        }
                    }
                }
                for messages in logs {
                    let messages = messages.as_array().unwrap();
                    for message in messages {
                        let message = message.as_str().unwrap();
                        println!("{}", message);
                    }
                }
                Err((client, Error::TestResultsNotFoundInLogs))
            })
            .or_else(|(mut client, err)| client.close().then(move |_| Err(err)))
    }

    /// Returns url which runner will open.
    fn get_url_to_test(&self, test_path: &PathBuf) -> String {
        let filename = test_path.file_name().unwrap().to_str().unwrap();
        format!("http://{}/e2e-tests/{}", self.test_addr, filename)
    }
}

/// Returns urls to all helpers JS from `e2e-tests/helper`.
fn get_all_helpers_urls() -> Result<Vec<String>, IoError> {
    let mut test_path = crate::get_path_to_tests();
    let mut helpers = Vec::new();
    test_path.push("helper");
    for entry in std::fs::read_dir(test_path)? {
        let entry = entry?;
        let path = entry.path();
        helpers.push(path);
    }

    Ok(helpers
        .into_iter()
        .map(|f| {
            format!(
                "/e2e-tests/helper/{}",
                f.file_name().unwrap().to_str().unwrap()
            )
        })
        .collect())
}

/// Generate html for spec by `test_template.html` from root.
fn generate_test_html(test_name: &str) -> String {
    let dont_edit_warning = "<!--DON'T EDIT THIS FILE. THIS IS AUTOGENERATED \
                             FILE FOR TESTS-->"
        .to_string();
    let html_body =
        include_str!("../test_template.html").replace("{{{test}}}", test_name);

    let mut helpers_include = String::new();
    for helper_url in get_all_helpers_urls().unwrap() {
        helpers_include
            .push_str(&format!(r#"<script src="{}"></script>\n"#, helper_url));
    }
    let html_body = html_body.replace("<helpers/>", &helpers_include);

    format!("{}\n{}", dont_edit_warning, html_body)
}

/// Generate html and save it with same path as a spec but with extension
/// `.html`.
fn generate_and_save_test_html(test_path: &PathBuf) -> PathBuf {
    let test_html =
        generate_test_html(test_path.file_name().unwrap().to_str().unwrap());

    let html_test_file_path = test_path.with_extension("html");
    let mut file = File::create(&html_test_file_path).unwrap();
    file.write_all(test_html.as_bytes()).unwrap();

    html_test_file_path
}

/// This future resolve when div with ID `test-end` appear on page.
fn wait_for_test_end(
    client: Client,
) -> impl Future<Item = Client, Error = CmdError> {
    client
        .wait_for_find(Locator::Id("test-end"))
        .map(fantoccini::Element::client)
}

/// Get all paths to spec files from provided dir.
fn get_all_tests_paths(path_to_test_dir: &PathBuf) -> Vec<PathBuf> {
    let mut tests_paths = Vec::new();
    for entry in std::fs::read_dir(path_to_test_dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_file() {
            if let Some(ext) = path.extension() {
                if ext == "js" {
                    tests_paths.push(path);
                }
            }
        }
    }
    tests_paths
}

/// Returns browser capabilities based on arguments.
///
/// Currently check `--headless` flag and based on this run headed or headless
/// browser.
fn get_webdriver_capabilities(opts: &ArgMatches) -> Capabilities {
    let mut capabilities = Capabilities::new();

    let mut firefox_args = Vec::new();
    let mut chrome_args = vec![
        "--use-fake-device-for-media-stream",
        "--use-fake-ui-for-media-stream",
        "--disable-web-security",
        "--disable-dev-shm-usage",
        "--no-sandbox",
    ];
    if opts.is_present("headless") {
        firefox_args.push("--headless");
        chrome_args.push("--headless");
    }

    let firefox_settings = json!({
        "prefs": {
            "media.navigator.streams.fake": true,
            "media.navigator.permission.disabled": true,
            "media.autoplay.enabled": true,
            "media.autoplay.enabled.user-gestures-needed ": false,
            "media.autoplay.ask-permission": false,
            "media.autoplay.default": 0,
        },
        "args": firefox_args
    });
    capabilities.insert("moz:firefoxOptions".to_string(), firefox_settings);

    let chrome_settings = json!({ "args": chrome_args });
    capabilities.insert("goog:chromeOptions".to_string(), chrome_settings);

    capabilities
}