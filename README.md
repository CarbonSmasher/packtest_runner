# PackTest Runner
This is a simple tool to help with running the [PackTest](https://github.com/misode/packtest) Minecraft datapack testing framework. It will start a Fabric server in the `packtest_launch` directory with the PackTest mod loaded, then automatically run the tests in the packs you specify. It will return an exit code based on the success of the tests when finished.

## Usage

### Command line
`packtest_runner [OPTIONS...] <PACK1> [PACK2] [PACK3] ...`

#### Options
 - `--version`: The Minecraft version to use. PackTest only supports 1.19.4, and this is the default.
 - `--comma-separate`: If set, will read the packs from the first argument specified, and split it by commas into multiple packs. Do not add spaces between the packs.
 - `--github`: Shows extra messages in the output for GitHub actions to use.

### GitHub Actions
You can use this repository as a GitHub action step to test your datapack in your CI/CD pipeline.

#### Example (from [https://github.com/misode/packtest-runner-example]())
```yml
name: Run tests

on: [push, workflow_dispatch]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: CarbonSmasher/packtest_runner@d7efd0cd6bc09d67f95f695ef559d79b3b2a08c0
        with:
          packs: 'failing,succeeding.zip'
```
