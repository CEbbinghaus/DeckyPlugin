# Contributing

## Contributing Guidelines

Thank you for considering contributing to the MicroSDeck project! We welcome any and all contributions, no matter how big or small. Whether you're fixing a typo or refactoring the entire backend, your contributions are valuable to us.

To ensure a positive and inclusive environment, we kindly ask all contributors to adhere to the following guidelines:

### Code of Conduct

Please review and abide by our [Code of Conduct](link-to-code-of-conduct) in all discussions and interactions related to MicroSDeck, both within and outside of GitHub. We strive to maintain a safe and respectful space for everyone involved.

## Getting Started
This project defines all of its tool dependencies in the `.mise.toml` file in the project root. The [Mise](https://mise.jdx.dev/) tool can be used to install them all with `mise install`. Alternatively it can be used as a reference as to which versions are required. Mise also offers tasks which take over some of the responsibility from `pnpm`, They can be run with `mise run [build|copy|upload]`, Alternatively the build script (located at `/util/build.mjs`) can be invoked with either `./build.sh` or `node --no-warnings=ExperimentalWarning util/build.mjs`, use `-h` to get a list of options.

After the project has built the outputs should have been collected into the `/build` directory which can be zipped up and uploaded to the steamdeck, For this the build script with --upload or the mise `upload` task can be used. 

## Documentation

If you would like to contribute to the MicroSDeck documentation, please ensure that your changes follow the guidelines outlined in the [docs/README.md](docs/README.md).

## Project Layout

### /src
This is the UI directory in which all logic the user interacts with is present. As every other Decky plugin it is written in Typescript using React as that is what the Steam Client uses for its rendering.

### /lib
The lib directory contains all of the glue that lets the frontend and backend communicate. It also contains all the type definitions of the backend types and its types are published as an NPM package to allow other plugins to integrate with MicroSDeck.

### /backend
This is the backend written in Rust. It is responsible for monitoring the filesystem and providing the database and API. It is built with docker from the decky cli which dissallows file binds from outside the backend directory so the version file is located there too.

### /version
The version file is used to populate all the packages to ensure consistency. The `/util/versioning.mjs` script (also called from `/util/build.mjs`) is responsible for updating the package.json versions. Since the backend cannot access files outside of its own folder the source must be `/backend/version` while `/version` is a symbolic link to allow for easy editing. 

### /docs
This directory contains all of the MicroSDeck documentation. It is traversed/parsed/rendered in during build by `/src/pages/docs.codegen` which then embeds the docs within the plugin to be rendered by the user. The format is in [mdx](https://mdxjs.com/) which allows for React components to be interleaved with the Markdown making the docs all the more interactive. 