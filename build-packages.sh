#!/usr/bin/env bash
set -e

docker image build --file build-debian-amd64.Dockerfile --tag bandwhichd-agent:debian .
docker container create --name bandwhichd-agent-debian bandwhichd-agent:debian
docker container cp bandwhichd-agent-debian:/bandwhichd-agent.deb ./bandwhichd-agent.0.37.0-1_amd64.deb || true
docker container rm bandwhichd-agent-debian

docker image build --file build-suse-amd64.Dockerfile --tag bandwhichd-agent:suse .
docker container create --name bandwhichd-agent-suse bandwhichd-agent:suse
docker container cp bandwhichd-agent-suse:/usr/src/packages/RPMS/x86_64/bandwhichd-agent-0.37.0-1.x86_64.rpm ./bandwhichd-agent-0.37.0-1.x86_64.rpm || true
docker container rm bandwhichd-agent-suse
