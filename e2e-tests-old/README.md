Medea E2E tests
==============

Supposed to be used for e2e testing.

Contains app with function for e2e testing and the testing code themselves.

For testing is using [cypress] javascript framework.

At this moment, tests can be run only
in the browser `chromium` because [cypress]'s electron
and chrome is too old for APIs which we use.

Test application exposing on `8082` port.

For __run tests__ execute `make test.e2e`.

__Don't forget to run `make deps`.__

## Run e2e tests manually:
1. start e2e testing app (`npm run start` in this dir)
2. start medea (`cargo run` in the root dir)
3. start coturn (`make up.coturn` in the root dir)
4. start control API mock server (`cargo run -p control-api-mock` in the root dir)
5. start [cypress] tests (`yarn run cypress run -b chromium` in this dir)


[cypress]: https://www.cypress.io/
