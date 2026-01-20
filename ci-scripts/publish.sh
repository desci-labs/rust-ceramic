#!/usr/bin/env bash

# Build and publish a docker image run running ceramic-one
#
# DOCKER_PASSWORD must be set
# Use:
#
#   export DOCKER_PASSWORD=$(aws ecr-public get-login-password --region us-east-2)
#   echo "${DOCKER_PASSWORD}" | docker login --username AWS --password-stdin 523044037273.dkr.ecr.us-east-2.amazonaws.com
#
# to login to docker. That password will be valid for 12h.

BUILD_MODE=${BUILD_MODE-release}
IMAGE_NAME=${IMAGE_NAME-523044037273.dkr.ecr.us-east-2.amazonaws.com/ceramic-one}

docker buildx build --load --build-arg="BUILD_MODE=$BUILD_MODE" -t ceramic-one .

if [[ -n "$SHA" ]]; then
  docker tag ceramic-one:latest "${IMAGE_NAME}":"$SHA"
fi
if [[ -n "$SHA_TAG" ]]; then
  docker tag ceramic-one:latest "${IMAGE_NAME}":"$SHA_TAG"
fi
if [[ -n "$RELEASE_TAG" ]]; then
  docker tag ceramic-one:latest "${IMAGE_NAME}":"$RELEASE_TAG"
fi
if [[ "$TAG_LATEST" == "true" ]]; then
  docker tag ceramic-one:latest "${IMAGE_NAME}":latest
fi
if [[ -n "$CUSTOM_TAG" ]]; then
  docker tag ceramic-one:latest "${IMAGE_NAME}":"$CUSTOM_TAG"
fi

if [ "$NO_PUSH" != "true" ]
then
    docker push -a "${IMAGE_NAME}"
fi

