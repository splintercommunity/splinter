# Copyright 2018-2022 Cargill Incorporated
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

FROM ubuntu:bionic

RUN apt-get update \
 && apt-get install -qy --no-install-recommends \
    ca-certificates \
    tar \
    wget \
 && wget -q https://dl.influxdata.com/telegraf/releases/telegraf_1.17.2-1_amd64.deb \
 && dpkg -i telegraf_1.17.2-1_amd64.deb \
 && rm -r /var/lib/apt/lists/*

WORKDIR /project
