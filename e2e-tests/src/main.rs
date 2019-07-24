use fantoccini::Client;
use futures::Future as _;
use serde_json::json;
use std::{
    fs::{canonicalize, File},
    io::prelude::*,
    path::PathBuf,
};
use webdriver::capabilities::Capabilities;

pub fn generate_html(test_js: &str) -> String {
    format!(r#"
<html>
<head>
    <meta http-equiv="Content-type" content="text/html; charset=utf-8"/>
    <title>Medea's e2e test</title>
    <link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/mocha/6.2.0/mocha.css">
</head>
<body>
<div id="mocha"></div>
<script src="https://cdnjs.cloudflare.com/ajax/libs/mocha/6.2.0/mocha.js"></script>
<script src="https://cdnjs.cloudflare.com/ajax/libs/chai/4.2.0/chai.js"></script>
<script>mocha.setup('bdd')</script>
<script>
{}
</script>
<script>mocha.run()</script>
</body>
</html>
    "#, test_js)
}

pub fn generate_html_test(test_path: &PathBuf) {
    let mut file = File::open(test_path).unwrap();
    let mut content = String::new();
    file.read_to_string(&mut content);
    let mut file = File::create("test.html").unwrap();
    let test_html = generate_html(&content);
    file.write_all(test_html.as_bytes()).unwrap();
}

fn main() {
    let path_to_tests = std::env::args().skip(1).next().unwrap();
    let path_to_tests = PathBuf::from(path_to_tests);
    let path_to_tests = canonicalize(path_to_tests).unwrap();

    generate_html_test(&path_to_tests);

    let mut capabilities = Capabilities::new();
    let firefox_settings = json!({
        "prefs": {
            "media.navigator.streams.fake": true
        }
    });
    capabilities.insert("moz:firefoxOptions".to_string(), firefox_settings);

    // TODO: chrome
    {
        let chrome_settings = json!({
            "args": [
                "--use-fake-device-for-media-stream",
                "--use-fake-ui-for-media-stream"
            ]
        });
        capabilities.insert("goog:chromeOptions".to_string(), chrome_settings);
    }

    if path_to_tests.is_dir() {
        unimplemented!("dir")
    } else {
        let client =
            Client::with_capabilities("http://localhost:9515", capabilities);
//        let test_url = format!("file://{}", path_to_tests.display());
        let test_url = "file:///home/relmay/Projects/work/medea/e2e-tests/test.html";

        tokio::run(
            client
                .map_err(|e| panic!("{:?}", e))
                .and_then(move |client| client.goto(&test_url))
                .map(|client| {
                    std::thread::sleep_ms(5000);
                })
                .map_err(|_| ()),
        );
    }
}
