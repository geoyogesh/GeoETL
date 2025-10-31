# GeoETL Documentation Website

This is the documentation website for [GeoETL](https://github.com/geoyogesh/geoetl), built using [Docusaurus](https://docusaurus.io/), a modern static website generator.

## Installation

```bash
yarn
```

## Local Development

```bash
yarn start
```

This command starts a local development server and opens up a browser window. Most changes are reflected live without having to restart the server.

## Build

```bash
yarn build
```

This command generates static content into the `build` directory and can be served using any static contents hosting service.

## Serve

```bash
yarn serve
```

This command serves the built website locally.

## Re-building the site

If you are facing issues with the live reload or content not updating, you can try re-building the site from scratch.

```bash
# 1. Clear the Docusaurus cache
yarn clear

# 2. Re-build the site from scratch
yarn build

# 3. Serve the new build
yarn serve
```

## Deployment

Using SSH:

```bash
USE_SSH=true yarn deploy
```

Not using SSH:

```bash
GIT_USER=<Your GitHub username> yarn deploy
```

If you are using GitHub pages for hosting, this command is a convenient way to build the website and push to the `gh-pages` branch.
