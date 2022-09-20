# trace-recorder-parser &emsp; ![ci] [![crates.io]](https://crates.io/crates/trace-recorder-parser) [![docs.rs]](https://docs.rs/trace-recorder-parser)

A Rust library to parse Percepio's [TraceRecorder](https://github.com/percepio/TraceRecorderSource) data.

Supports the following kernel ports:
* FreeRTOS
  - snapshot protocol, format version 6
  - streaming protocol format version 10

## LICENSE

See [LICENSE](./LICENSE) for more details.

Copyright 2022 [Auxon Corporation](https://auxon.io)

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

[http://www.apache.org/licenses/LICENSE-2.0](http://www.apache.org/licenses/LICENSE-2.0)

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.

[ci]: https://github.com/auxoncorp/trace-recorder-parser/workflows/CI/badge.svg
[crates.io]: https://img.shields.io/crates/v/trace-recorder-parser.svg
[docs.rs]: https://docs.rs/trace-recorder-parser/badge.svg
