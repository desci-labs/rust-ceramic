#!/usr/bin/env bash

# Build and publish a docker image to run the tests suite
#
# DOCKER_PASSWORD must be set
# Use:
#
# aws ecr-public get-login-password --region us-east-1 | docker login --username AWS --password-stdin public.ecr.aws/m3e8d9d6
#
# to login to docker. That password will be valid for 12h.

# Change to the suite directory
cd $(dirname $0)/../suite

tag=${1-latest}
BUILD_PROFILE=${BUILD_PROFILE-release}
IMAGE_NAME=${IMAGE_NAME-public.ecr.aws/m3e8d9d6/desci-labs/ceramic-tests-suite}

PUSH_ARGS="--push"
if [ "$NO_PUSH" = "true" ]
then
    PUSH_ARGS=""
fi

CACHE_ARGS=""
if [ -n "$ACTIONS_CACHE_URL" ]
then
    # Use Github Actions cache
    CACHE_ARGS="--cache-to type=gha --cache-from type=gha"
fi

docker buildx build \
    $PUSH_ARGS \
    $CACHE_ARGS \
    --build-arg BUILD_PROFILE=${BUILD_PROFILE} \
    -t ${IMAGE_NAME}:$tag \
    -f Dockerfile \
    ../.. # Use the repo root as the context so that we can include the sdk source
