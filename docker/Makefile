# help
define cmdline
Please choose one of the specific commands:
  - init  : create and initialize the docker image
  - once  : probe once into the docker image and discard the changes
  - build : build the project from scratch
endef
export cmdline

help:
	@echo "$$cmdline"

# paths
mkfile_path := $(abspath $(lastword $(MAKEFILE_LIST)))
current_dir := $(dir $(mkfile_path))
project_dir := $(realpath $(current_dir)/..)

# variables
tag = libra

# targets
init:
	@docker build . \
		--no-cache \
		-t $(tag)
	@docker run --rm \
		-v $(project_dir):/project \
		$(tag) \
		rm -rf /project/studio/docker

once:
	@docker run --rm \
		-v $(project_dir):/project \
		--interactive --tty \
		$(tag) \
		bash

build:
	@docker run --rm \
		-v $(project_dir):/project \
		--interactive --tty \
		$(tag) \
		bash /project/docker/script/build.sh

