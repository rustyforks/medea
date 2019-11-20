###############################
# Common defaults/definitions #
###############################

comma := ,

# Checks two given strings for equality.
eq = $(if $(or $(1),$(2)),$(and $(findstring $(1),$(2)),\
                                $(findstring $(2),$(1))),1)




######################
# Project parameters #
######################

MEDEA_IMAGE_NAME := $(strip \
	$(shell grep 'COMPOSE_IMAGE_NAME=' .env | cut -d '=' -f2))
DEMO_IMAGE_NAME := instrumentisto/medea-demo
CONTROL_MOCK_IMAGE_NAME := instrumentisto/medea-control-api-mock

RUST_VER := 1.39
CHROME_VERSION := 78.0
FIREFOX_VERSION := 70.0

crate-dir = .
ifeq ($(crate),medea-jason)
crate-dir = jason
endif
ifeq ($(crate),medea-client-api-proto)
crate-dir = proto/client-api
endif
ifeq ($(crate),medea-control-api-proto)
crate-dir = proto/control-api
endif
ifeq ($(crate),medea-control-api-mock)
crate-dir = mock/control-api
endif
ifeq ($(crate),medea-macro)
crate-dir = crates/medea-macro
endif




###########
# Aliases #
###########

# Build all project executables.
#
# Usage:
#	make build

build: build.medea docker.build.medea build.jason


build.medea:
	@make cargo.build crate=medea debug=$(debug) dockerized=$(dockerized)


build.jason:
	@make cargo.build crate=medea-jason debug=$(debug) dockerized=$(dockerized)


# Resolve all project dependencies.
#
# Usage:
#	make deps

deps: cargo yarn


docs: docs.rust


down: down.dev


fmt: cargo.fmt


lint: cargo.lint


# Build and publish project crate everywhere.
#
# Usage:
#	make release crate=(medea|medea-jason|<crate-name>)
#	             [publish=(no|yes)]

release: release.crates release.npm


# Run all project tests.
#
# Usage:
#	make test

test:
	@make test.unit
	@make test.integration up=yes dockerized=no
	@make test.e2e browser=chrome
	@make test.e2e browser=firefox


up: up.dev




####################
# Running commands #
####################

# Stop non-dockerized Control API mock server.
#
# Usage:
#   make down.control

down.control:
	kill -9 $(pidof medea-control-api-mock)


down.coturn: docker.down.coturn


down.demo: docker.down.demo


# Stop all processes in Medea and Jason development environment.
#
# Usage:
#	make down.dev

down.dev:
	@make docker.down.medea dockerized=no
	@make docker.down.medea dockerized=yes
	@make docker.down.coturn


# Stop all services needed for e2e testing of medea in browsers.
#
# Usage:
#   make down.e2e.services [dockerized=(yes|no)] [coturn=(yes|no)]

down.e2e.services:
ifeq ($(dockerized),no)
	-kill $$(cat /tmp/e2e_medea.pid)
	-kill $$(cat /tmp/e2e_control_api_mock.pid)
	-rm -f /tmp/e2e_medea.pid \
		/tmp/e2e_control_api_mock.pid
ifneq ($(coturn),no)
	-@make down.coturn
endif
else
	-docker container stop $$(cat /tmp/control-api-mock.docker.uid)
	-docker container stop $$(cat /tmp/medea.docker.uid)
	-rm -f /tmp/control-api-mock.docker.uid /tmp/medea.docker.uid

	-@make down.coturn
endif


down.medea: docker.down.medea


# Run Control API mock server.
#
# Usage:
#  make up.control

up.control:
	cargo run -p medea-control-api-mock


up.coturn: docker.up.coturn


up.demo: docker.up.demo


# Run Medea and Jason development environment.
#
# Usage:
#	make up.dev

up.dev: up.coturn
	$(MAKE) -j3 up.jason docker.up.medea up.control


up.medea: docker.up.medea


# Run Jason E2E demo in development mode.
#
# Usage:
#	make up.jason

up.jason:
	npm run start --prefix=jason/e2e-demo


# Run Control API mock server.
#
# Usage:
#  make up.control-api-mock

up.control-api-mock:
	cargo run -p control-api-mock


# Start webdriver.
#
# Usage:
#   make up.webdriver [browser=(chrome|firefox)]
#                     [dockerized=(yes|no)]

up.webdriver: down.webdriver
ifeq ($(browser),firefox)
ifeq ($(dockerized),no)
	geckodriver &
else
	docker run --rm -d --shm-size 256m --name medea-test-ff \
		--network=host alexlapa/geckodriver:${FIREFOX_VERSION}
endif
else
ifeq ($(dockerized),no)
	chromedriver --port=4444 &
else
	docker run --rm -d --name medea-test-chrome \
		--network=host selenoid/chrome:$(CHROME_VERSION)
endif
endif




##################
# Cargo commands #
##################

# Resolve Cargo project dependencies.
#
# Usage:
#	make cargo [cmd=(fetch|<cargo-cmd>)]

cargo:
	cargo $(if $(call eq,$(cmd),),fetch,$(cmd))


# Build medea's related crates.
#
# Usage:
#	make cargo.build [crate=(@all|medea|medea-jason)]
#	                 [debug=(yes|no)]
#	                 [dockerized=(no|yes)]

cargo-build-crate = $(if $(call eq,$(crate),),@all,$(crate))

cargo.build:
ifeq ($(cargo-build-crate),@all)
	@make build crate=medea
	@make build crate=medea-jason
endif
ifeq ($(cargo-build-crate),medea)
ifeq ($(dockerized),yes)
	docker run --rm -v "$(PWD)":/app -w /app \
		-u $(shell id -u):$(shell id -g) \
		-v "$(HOME)/.cargo/registry":/usr/local/cargo/registry \
		rust:$(RUST_VER) \
			make cargo.build crate=$(cargo-build-crate) \
			                 debug=$(debug) dockerized=no
else
	cargo build --bin medea $(if $(call eq,$(debug),no),--release,)
endif
endif
ifeq ($(cargo-build-crate),medea-jason)
ifeq ($(dockerized),yes)
	docker run --rm --network=host -v "$(PWD)":/app -w /app \
		-u $(shell id -u):$(shell id -g) \
		-v "$(HOME)/.cargo/registry":/usr/local/cargo/registry \
		-v "$(HOME):$(HOME)" \
		-e XDG_CACHE_HOME=$(HOME) \
		rust:$(RUST_VER) \
			make cargo.build crate=$(cargo-build-crate) \
			                 debug=$(debug) dockerized=no \
			                 pre-install=yes
else
ifeq ($(pre-install),yes)
	curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
endif
	@rm -rf $(crate-dir)/pkg/
	wasm-pack build -t web $(crate-dir)/
endif
endif


# Format Rust sources with rustfmt.
#
# Usage:
#	make cargo.fmt [check=(no|yes)]

cargo.fmt:
	cargo +nightly fmt --all $(if $(call eq,$(check),yes),-- --check,)


# Generate Rust sources with Cargo's build.rs script.
#
# Usage:
#	make cargo.gen crate=medea-control-api-proto

cargo.gen:
ifeq ($(crate),medea-control-api-proto)
	@rm -rf $(crate-dir)/src/grpc/api*.rs
	cd $(crate-dir)/ && \
	cargo build
endif


# Lint Rust sources with clippy.
#
# Usage:
#	make cargo.lint

cargo.lint:
	cargo clippy --all -- -D clippy::pedantic -D warnings




#################
# Yarn commands #
#################

# Resolve NPM project dependencies with Yarn.
#
# Optional 'cmd' parameter may be used for handy usage of docker-wrapped Yarn,
# for example: make yarn cmd='upgrade'
#
# Usage:
#	make yarn [cmd=('install --pure-lockfile'|<yarn-cmd>)]
#	          [proj=(e2e|demo)]
#	          [dockerized=(yes|no)]

yarn-cmd = $(if $(call eq,$(cmd),),install --pure-lockfile,$(cmd))
yarn-proj-dir = $(if $(call eq,$(proj),demo),jason/demo,jason/e2e-demo)

yarn:
ifneq ($(dockerized),no)
	docker run --rm --network=host -v "$(PWD)":/app -w /app \
	           -u $(shell id -u):$(shell id -g) \
		node:latest \
			make yarn cmd='$(yarn-cmd)' proj=$(proj) dockerized=no
else
	yarn --cwd=$(yarn-proj-dir) $(yarn-cmd)
endif




##########################
# Documentation commands #
##########################

# Generate project documentation of Rust sources.
#
# Usage:
#	make docs.rust [crate=(@all|medea|medea-jason|<crate-name>)]
#	               [open=(yes|no)] [clean=(no|yes)]

docs-rust-crate = $(if $(call eq,$(crate),),@all,$(crate))

docs.rust:
ifeq ($(clean),yes)
	@rm -rf target/doc/
endif
	cargo +nightly doc \
		$(if $(call eq,$(docs-rust-crate),@all),--all,-p $(docs-rust-crate)) \
		--no-deps \
		$(if $(call eq,$(open),no),,--open)




####################
# Testing commands #
####################

# Run Rust unit tests of project.
#
# Usage:
#	make test.unit [crate=(@all|medea|<crate-name>)]
#	               [crate=medea-jason [browser=(chrome|firefox|default)]]

test-unit-crate = $(if $(call eq,$(crate),),@all,$(crate))
webdriver-env = $(if $(call eq,$(browser),firefox),GECKO,CHROME)DRIVER_REMOTE

test.unit:
ifeq ($(test-unit-crate),@all)
	@make test.unit crate=medea-macro
	@make test.unit crate=medea-client-api-proto
	@make test.unit crate=medea-jason
	@make test.unit crate=medea
else
ifeq ($(test-unit-crate),medea)
	cargo test --lib --bin medea
else
ifeq ($(crate),medea-jason)
ifeq ($(browser),default)
	cd $(crate-dir)/ && \
	cargo test --target wasm32-unknown-unknown --features mockable
else
	@make docker.up.webdriver browser=$(browser)
	sleep 10
	cd $(crate-dir)/ && \
	$(webdriver-env)="http://127.0.0.1:4444" \
	cargo test --target wasm32-unknown-unknown --features mockable
	@make docker.down.webdriver browser=$(browser)
endif
else
	cd $(crate-dir)/ && \
	cargo test -p $(test-unit-crate)
endif
endif
endif


# Run Rust E2E tests of project.
#
# Usage:
# 	make test.integration [up=no]
#	              [up=yes [dockerized=no [debug=(yes|no)]]
#	                      [dockerized=yes [TAG=(dev|<docker-tag>)]
#	                                      [registry=<registry-host>]
#	                                      [log=(no|yes)]]
#	                      [wait=(5|<seconds>)]]

test-integration-env = RUST_BACKTRACE=1 \
	$(if $(call eq,$(log),yes),,RUST_LOG=warn) \
	MEDEA_CONTROL_API__STATIC_SPECS_DIR=tests/specs/

test.integration:
ifeq ($(up),yes)
	make docker.up.coturn background=yes
	env $(test-integration-env) \
	make docker.up.medea debug=$(debug) background=yes log=$(log) \
	                     dockerized=$(dockerized) \
	                     TAG=$(TAG) registry=$(registry)
	sleep $(if $(call eq,$(wait),),5,$(wait))
endif
	RUST_BACKTRACE=1 cargo test --test e2e
ifeq ($(up),yes)
	-make down
endif


# Run E2E project tests.
#
# You can use 'wait-on-fail=yes' for debug. This flag will
# don't close browser immediately on test fail but
# wait for user input and close browser only when
# you press <Enter>. 'wait-on-fail=yes' usage make sense only
# with 'dockerized=no' and 'headless=no' flags.
#
# Usage:
#   make test.e2e [dockerized=(yes|no)]
#                 [headless=(yes|no)]
#                 [browser=(chrome|firefox)]
#                 [wait-on-fail=(no|yes)]

test.e2e: up.e2e.services up.webdriver
	sleep 3
	$(if $(call eq,$(dockerized),no),,$(run-medea-container)) cargo run -p e2e-tests-runner -- \
		-w http://localhost:4444 \
		-f localhost:$(test-runner-port) \
		$(if $(call eq,$(headless),no),,--headless) \
		$(if $(call eq,$(wait-on-fail),yes),--wait-on-fail,)
	@make down.webdriver
	@make down.e2e.services


# Start services needed for e2e tests of medea in browsers.
# If logs set to "yes" then medea print all logs to stdout.
#
# Usage:
# 	make test.e2e [dockerized=(YES|no)] [logs=(yes|NO)] [coturn=(YES|no)]

medea-env = RUST_BACKTRACE=1 \
	MEDEA_SERVER.HTTP.BIND_PORT=8081 \
	$(if $(call eq,$(logs),yes),,RUST_LOG=warn) \
	MEDEA_SERVER.HTTP.STATIC_SPECS_PATH=tests/specs

chromedriver-port = 50000
geckodriver-port = 50001
test-runner-port = 51000

run-medea-command = docker run --rm --network=host -v "$(PWD)":/app -w /app \
                    	--env XDG_CACHE_HOME=$(HOME) \
                    	--env RUST_BACKTRACE=1 \
						-u $(shell id -u):$(shell id -g) \
                    	-v "$(HOME)/.cargo/registry":/usr/local/cargo/registry \
                    	-v "$(HOME):$(HOME)" \
                    	-v "$(PWD)/target":/app/target
run-medea-container-d =  $(run-medea-command) -d alexlapa/medea-build:dev
run-medea-container = $(run-medea-command) alexlapa/medea-build:dev

up.e2e.services:
ifneq ($(dockerized),no)
	mkdir -p .cache target ~/.cargo/registry
endif
ifneq ($(coturn),no)
	@make up.coturn
endif
ifeq ($(dockerized),no)
	cargo build $(if $(call eq,$(release),yes),--release)
	cargo build -p control-api-mock
	pushd jason && wasm-pack build --target web --out-dir ../.cache/jason-pkg && popd

	env $(if $(call eq,$(logs),yes),,RUST_LOG=warn) cargo run --bin medea \
		$(if $(call eq,$(release),yes),--release) & \
		echo $$! > /tmp/e2e_medea.pid
	env RUST_LOG=warn cargo run -p control-api-mock & \
		echo $$! > /tmp/e2e_control_api_mock.pid

	cargo build -p e2e-tests-runner
else
	@make down.medea dockerized=yes
	@make down.medea dockerized=no
	@make up.coturn

	$(run-medea-container) sh -c "cd jason && RUST_LOG=info wasm-pack build --target web --out-dir ../.cache/jason-pkg"

	$(run-medea-container) make build.medea optimized=yes
	$(run-medea-container-d) cargo run --release > /tmp/medea.docker.uid

	$(run-medea-container) cargo build -p control-api-mock
	$(run-medea-container-d) cargo run -p control-api-mock > /tmp/control-api-mock.docker.uid

	$(run-medea-container) cargo build -p e2e-tests-runner
endif




######################
# Releasing commands #
######################

# Build and publish project crate to crates.io.
#
# Usage:
#	make release.crates crate=(medea|medea-jason|<crate-name>)
#	                    [token=($CARGO_TOKEN|<cargo-token>)]
#	                    [publish=(no|yes)]

release-crates-token = $(if $(call eq,$(token),),${CARGO_TOKEN},$(token))

release.crates:
ifneq ($(filter $(crate),medea medea-jason medea-client-api-proto medea-macro),)
	cd $(crate-dir)/ && \
	$(if $(call eq,$(publish),yes),\
		cargo publish --token $(release-crates-token) ,\
		cargo package --allow-dirty )
endif


release.helm: helm.package.release


# Build and publish project crate to NPM.
#
# Usage:
#	make release.npm crate=medea-jason
#	                 [publish=(no|yes)]

release.npm:
ifneq ($(filter $(crate),medea-jason),)
	@make cargo.build crate=$(crate) debug=no dockerized=no
ifeq ($(publish),yes)
	wasm-pack publish $(crate-dir)/
endif
endif




###################
# Docker commands #
###################

docker-env = $(strip $(if $(call eq,$(minikube),yes),\
	$(subst export,,$(shell minikube docker-env | cut -d '\#' -f1)),))

# Authenticate to Container Registry where project Docker images are stored.
#
# Usage:
#	make docker.auth [registry=<registry-host>]
#	                 [user=<username>] [pass-stdin=(no|yes)]

docker.auth:
	docker login $(registry) \
		$(if $(call eq,$(user),),,--username=$(user)) \
		$(if $(call eq,$(pass-stdin),yes),--password-stdin,)


# Build Docker image for Control API mock server.
#
# Usage:
#	make docker.build.control [TAG=(dev|<tag>)] [debug=(yes|no)]
#	                          [minikube=(no|yes)]

docker.build.control:
	$(docker-env) \
	docker build $(if $(call eq,$(minikube),yes),,--network=host) \
		--build-arg rust_ver=$(RUST_VER) \
		--build-arg rustc_mode=$(if $(call eq,$(debug),no),release,debug) \
		--build-arg rustc_opts=$(if $(call eq,$(debug),no),--release,) \
		-t $(CONTROL_MOCK_IMAGE_NAME):$(if $(call eq,$(TAG),),dev,$(TAG)) \
		-f mock/control-api/Dockerfile .


# Build Docker image for demo application.
#
# Usage:
#	make docker.build.demo [TAG=(dev|<tag>)]
#	                       [minikube=(no|yes)]

docker-build-demo-image-name = $(DEMO_IMAGE_NAME)

docker.build.demo:
ifeq ($(TAG),edge)
	$(docker-env) \
	docker build $(if $(call eq,$(minikube),yes),,--network=host) --force-rm \
		--build-arg rust_ver=$(RUST_VER) \
		-t $(docker-build-demo-image-name):$(TAG) \
		-f jason/Dockerfile .
else
	@make yarn proj=demo
	$(docker-env) \
	docker build $(if $(call eq,$(minikube),yes),,--network=host) --force-rm \
		-t $(docker-build-demo-image-name):$(if $(call eq,$(TAG),),dev,$(TAG)) \
		jason/demo
endif


# Build REST Control API mock server.
#
# Usage:
#   make docker.build.control-api-mock

docker.build.control-api-mock:
	docker build -t instrumentisto/medea-control-api-mock:dev \
		-f crates/control-api-mock/Dockerfile \
		--build-arg medea_build_image=$(medea-build-image) \
		.


# Build medea project Docker image.
#
# Usage:
#	make docker.build.medea [TAG=(dev|<tag>)] [registry=<registry-host>]
#	                        [debug=(yes|no)]
#	                        [no-cache=(no|yes)]
#	                        [minikube=(no|yes)]

docker-build-medea-image-name = $(strip \
	$(if $(call eq,$(registry),),,$(registry)/)$(MEDEA_IMAGE_NAME))

docker.build.medea:
	$(docker-env) \
	docker build $(if $(call eq,$(minikube),yes),,--network=host) --force-rm \
		$(if $(call eq,$(no-cache),yes),\
			--no-cache --pull,) \
		$(if $(call eq,$(IMAGE),),\
			--build-arg rust_ver=$(RUST_VER) \
			--build-arg rustc_mode=$(if \
				$(call eq,$(debug),no),release,debug) \
			--build-arg rustc_opts=$(if \
				$(call eq,$(debug),no),--release,),) \
		-t $(docker-build-medea-image-name):$(if $(call eq,$(TAG),),dev,$(TAG)) .


# Stop dockerized Control API mock server and remove all related containers.
#
# Usage:
#   make docker.down.control

docker.down.control:
	-docker stop medea-control-api-mock


# Stop Coturn STUN/TURN server in Docker Compose environment
# and remove all related containers.
#
# Usage:
# 	make docker.down.coturn

docker.down.coturn:
	docker-compose -f docker-compose.coturn.yml down --rmi=local -v


# Stop demo application in Docker Compose environment
# and remove all related containers.
#
# Usage:
#	make docker.down.demo

docker.down.demo:
	docker-compose -f jason/demo/docker-compose.yml down --rmi=local -v


# Stop Medea media server in Docker Compose environment
# and remove all related containers.
#
# Usage:
# 	make docker.down.medea [dockerized=(no|yes)]

docker.down.medea:
ifeq ($(dockerized),yes)
	docker-compose -f docker-compose.medea.yml down --rmi=local -v
else
	-killall medea
endif


# Stop dockerized WebDriver and remove all related containers.
#
# Usage:
#   make docker.down.webdriver [browser=(chrome|firefox)]

docker.down.webdriver:
	-docker stop medea-webdriver-$(if $(call eq,$(browser),),chrome,$(browser))


# Pull project Docker images from Container Registry.
#
# Usage:
#	make docker.pull [IMAGE=(medea|demo)] [registry=<registry-host>]
#	                 [TAGS=(@all|<t1>[,<t2>...])]
#	                 [minikube=(no|yes)]

docker-pull-image-name = $(strip \
	$(if $(call eq,$(registry),),,$(registry)/)$(strip \
	$(if $(call eq,$(IMAGE),demo),$(DEMO_IMAGE_NAME),$(MEDEA_IMAGE_NAME))))
docker-pull-tags = $(if $(call eq,$(TAGS),),@all,$(TAGS))

docker.pull:
ifeq ($(docker-pull-tags),@all)
	$(docker-env) \
	docker pull $(docker-pull-image-name) --all-tags
else
	$(foreach tag,$(subst $(comma), ,$(docker-pull-tags)),\
		$(call docker.pull.do,$(tag)))
endif
define docker.pull.do
	$(eval tag := $(strip $(1)))
	$(docker-env) \
	docker pull $(docker-pull-image-name):$(tag)
endef


# Push project Docker images to Container Registry.
#
# Usage:
#	make docker.push [IMAGE=(medea|demo)] [registry=<registry-host>]
#	                 [TAGS=(dev|<t1>[,<t2>...])]
#	                 [minikube=(no|yes)]

docker-push-image-name = $(strip \
	$(if $(call eq,$(registry),),,$(registry)/)$(strip \
	$(if $(call eq,$(IMAGE),demo),$(DEMO_IMAGE_NAME),$(MEDEA_IMAGE_NAME))))
docker-push-tags = $(if $(call eq,$(TAGS),),dev,$(TAGS))

docker.push:
	$(foreach tag,$(subst $(comma), ,$(docker-push-tags)),\
		$(call docker.push.do,$(tag)))
define docker.push.do
	$(eval tag := $(strip $(1)))
	$(docker-env) \
	docker push $(docker-push-image-name):$(tag)
endef


# Run dockerized Medea Control API mock server.
#
# Usage:
#   make docker.up.control [TAG=(dev|<docker-tag>)]

docker.up.control:
	docker run --rm -d --network=host \
		--name medea-control-api-mock \
		$(CONTROL_MOCK_IMAGE_NAME):$(if $(call eq,$(TAG),),dev,$(TAG))


# Run Coturn STUN/TURN server in Docker Compose environment.
#
# Usage:
#	make docker.up.coturn [background=(yes|no)]

docker.up.coturn: docker.down.coturn
	docker-compose -f docker-compose.coturn.yml up \
		$(if $(call eq,$(background),no),--abort-on-container-exit,-d)


# Run demo application in Docker Compose environment.
#
# Usage:
#	make docker.up.demo

docker.up.demo: docker.down.demo
	docker-compose -f jason/demo/docker-compose.yml up


# Run Medea media server in Docker Compose environment.
#
# Usage:
#	make docker.up.medea [dockerized=no [debug=(yes|no)] [background=(no|yes)]]
#	                     [dockerized=yes [TAG=(dev|<docker-tag>)]
#	                                     [registry=<registry-host>]]
#	                                     [background=no]
#	                                     [background=yes [log=(no|yes)]]]

docker-up-medea-image-name = $(strip \
	$(if $(call eq,$(registry),),,$(registry)/)$(MEDEA_IMAGE_NAME))
docker-up-medea-tag = $(if $(call eq,$(TAG),),dev,$(TAG))

docker.up.medea: docker.down.medea
ifeq ($(dockerized),yes)
	COMPOSE_IMAGE_NAME=$(docker-up-medea-image-name) \
	COMPOSE_IMAGE_VER=$(docker-up-medea-tag) \
	docker-compose -f docker-compose.medea.yml up \
		$(if $(call eq,$(background),yes),-d,--abort-on-container-exit)
ifeq ($(background),yes)
ifeq ($(log),yes)
	docker-compose -f docker-compose.medea.yml logs -f &
endif
endif
else
	cargo build --bin medea $(if $(call eq,$(debug),no),--release,)
	cargo run --bin medea $(if $(call eq,$(debug),no),--release,) \
		$(if $(call eq,$(background),yes),&,)
endif


# Run dockerized WebDriver.
#
# Usage:
#   make docker.up.webdriver [browser=(chrome|firefox)]

docker.up.webdriver:
	-@make docker.down.webdriver browser=chrome
	-@make docker.down.webdriver browser=firefox
ifeq ($(browser),firefox)
	docker run --rm -d --network=host --shm-size 512m \
		--name medea-webdriver-firefox \
		instrumentisto/geckodriver:$(FIREFOX_VERSION)
else
	docker run --rm -d --network=host \
		--name medea-webdriver-chrome \
		selenoid/chrome:$(CHROME_VERSION)
endif




##############################
# Helm and Minikube commands #
##############################

helm-cluster = $(if $(call eq,$(cluster),),minikube,$(cluster))
helm-namespace = $(if $(call eq,$(helm-cluster),minikube),kube,staging)-system
helm-cluster-args = $(strip \
	--kube-context=$(helm-cluster) --tiller-namespace=$(helm-namespace))

helm-chart = $(if $(call eq,$(chart),),medea-demo,$(chart))
helm-chart-dir = jason/demo/chart/medea-demo
helm-chart-vals-dir = jason/demo

helm-release = $(if $(call eq,$(release),),,$(release)-)$(helm-chart)
helm-release-namespace = $(strip \
	$(if $(call eq,$(helm-cluster),staging),staging,default))

# Run Helm command in context of concrete Kubernetes cluster.
#
# Usage:
#	make helm [cmd=(--help|'<command>')]
#	          [cluster=(minikube|staging)]

helm:
	helm $(helm-cluster-args) $(if $(call eq,$(cmd),),--help,$(cmd))


# Remove Helm release of project Helm chart from Kubernetes cluster.
#
# Usage:
#	make helm.down [chart=medea-demo] [release=<release-name>]
#	               [cluster=(minikube|staging)]

helm.down:
	$(if $(shell helm ls $(helm-cluster-args) | grep '$(helm-release)'),\
		helm del --purge $(helm-cluster-args) $(helm-release) ,\
		@echo "--> No '$(helm-release)' release found in $(helm-cluster) cluster")


# Upgrade (or initialize) Tiller (server side of Helm) of Minikube.
#
# Usage:
#	make helm.init [client-only=no [upgrade=(yes|no)]]
#	               [client-only=yes]

helm.init:
	helm init --wait \
		$(if $(call eq,$(client-only),yes),\
			--client-only,\
			--kube-context=minikube --tiller-namespace=kube-system \
				$(if $(call eq,$(upgrade),no),,--upgrade))


# Lint project Helm chart.
#
# Usage:
#	make helm.lint [chart=medea-demo]

helm.lint:
	helm lint $(helm-chart-dir)/


# List all Helm releases in Kubernetes cluster.
#
# Usage:
#	make helm.list [cluster=(minikube|staging)]

helm.list:
	helm ls $(helm-cluster-args)


# Build Helm package from project Helm chart.
#
# Usage:
#	make helm.package [chart=medea-demo]

helm-package-dir = .cache/helm/packages

helm.package:
	@rm -rf $(helm-package-dir)
	@mkdir -p $(helm-package-dir)/
	helm package --destination=$(helm-package-dir)/ $(helm-chart-dir)/


# Build and publish project Helm package to GitHub Pages.
#
# Usage:
#	make helm.package.release [chart=medea-demo] [build=(yes|no)]

helm-package-release-ver := $(strip $(shell \
	grep 'version: ' jason/demo/chart/medea-demo/Chart.yaml | cut -d ':' -f2))

helm.package.release:
ifneq ($(build),no)
	@make helm.package chart=$(helm-chart)
endif
	git fetch origin gh-pages:gh-pages
	git checkout gh-pages
	git reset --hard
	@mkdir -p charts/
	cp -rf $(helm-package-dir)/* charts/
	if [ -n "$$(git add -v charts/)" ]; then \
		helm repo index charts/ \
			--url=https://instrumentisto.github.io/medea/charts ; \
		git add -v charts/ ; \
		git commit -m \
			"Release $(helm-chart)-$(helm-package-release-ver) Helm chart" ; \
	fi
	git checkout -
	git push origin gh-pages


# Run project Helm chart in Kubernetes cluster as Helm release.
#
# Usage:
#	make helm.up [chart=medea-demo] [release=<release-name>]
#	             [cluster=minikube [rebuild=(no|yes) [no-cache=(no|yes)]]]
#	             [cluster=staging]
#	             [wait=(yes|no)]

helm.up:
ifeq ($(wildcard $(helm-chart-vals-dir)/my.$(helm-cluster).vals.yaml),)
	touch $(helm-chart-vals-dir)/my.$(helm-cluster).vals.yaml
endif
ifeq ($(helm-cluster),minikube)
ifeq ($(helm-chart),medea-demo)
ifeq ($(rebuild),yes)
	@make docker.build.demo minikube=yes TAG=dev
	@make docker.build.medea no-cache=$(no-cache) minikube=yes TAG=dev
	@make docker.build.control minikube=yes TAG=dev
endif
endif
endif
	helm upgrade --install --force $(helm-cluster-args) \
		$(helm-release) $(helm-chart-dir)/ \
			--namespace=$(helm-release-namespace) \
			--values=$(helm-chart-vals-dir)/$(helm-cluster).vals.yaml \
			--values=$(helm-chart-vals-dir)/my.$(helm-cluster).vals.yaml \
			--set server.deployment.revision=$(shell date +%s) \
			--set web-client.deployment.revision=$(shell date +%s) \
			$(if $(call eq,$(wait),no),,\
				--wait )


# Bootstrap Minikube cluster (local Kubernetes) for development environment.
#
# The bootsrap script is updated automatically to the latest version every day.
# For manual update use 'update=yes' command option.
#
# Usage:
#	make minikube.boot [update=(no|yes)]
#	                   [driver=(virtualbox|hyperkit|hyperv)]
#	                   [k8s-version=<kubernetes-version>]

minikube.boot:
ifeq ($(update),yes)
	$(call minikube.boot.download)
else
ifeq ($(wildcard $(HOME)/.minikube/bootstrap.sh),)
	$(call minikube.boot.download)
else
ifneq ($(shell find $(HOME)/.minikube/bootstrap.sh -mmin +1440),)
	$(call minikube.boot.download)
endif
endif
endif
	@$(if $(cal eq,$(driver),),,MINIKUBE_VM_DRIVER=$(driver)) \
	 $(if $(cal eq,$(k8s-version),),,MINIKUBE_K8S_VER=$(k8s-version)) \
		$(HOME)/.minikube/bootstrap.sh
define minikube.boot.download
	$()
	@mkdir -p $(HOME)/.minikube/
	@rm -f $(HOME)/.minikube/bootstrap.sh
	curl -fL -o $(HOME)/.minikube/bootstrap.sh \
		https://raw.githubusercontent.com/instrumentisto/toolchain/master/minikube/bootstrap.sh
	@chmod +x $(HOME)/.minikube/bootstrap.sh
endef




###################
# Protoc commands #
###################

# Rebuild gRPC protobuf specs for medea-control-api-proto.
#
# Usage:
#  make protoc.rebuild

protoc.rebuild:
	rm -f proto/control-api/src/grpc/control_api*.rs
	cargo build -p medea-control-api-proto



##################
# .PHONY section #
##################

.PHONY: build build.jason build.medea \
        cargo cargo.build cargo.fmt cargo.gen cargo.lint \
        docker.auth  docker.build.control docker.build.demo docker.build.medea \
        	docker.down.control docker.down.coturn docker.down.demo \
        	docker.down.medea docker.down.webdriver  \
        	docker.pull docker.push \
        	docker.up.control docker.up.coturn docker.up.demo docker.up.medea \
        	docker.up.webdriver \
        docs docs.rust \
        down down.control down.coturn down.demo down.dev down.medea \
        helm helm.down helm.init helm.lint helm.list \
        	helm.package helm.package.release helm.up \
        minikube.boot \
        protoc.rebuild \
        release release.crates release.helm release.npm \
        test test.e2e test.unit test.integration \
        up up.control up.coturn up.demo up.dev up.jason up.medea \
        yarn
