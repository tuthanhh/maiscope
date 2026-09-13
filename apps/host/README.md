[![Contributors][contributors-shield]][contributors-url]
[![Forks][forks-shield]][forks-url]
[![Stargazers][stars-shield]][stars-url]
[![Issues][issues-shield]][issues-url]
[![MIT License][license-shield]][license-url]



<!-- PROJECT LOGO -->
<br />
<div align="center">
<h3 align="center">maiscope</h3>

  <p align="center">
    A web song &amp; chart browser for maimai, built with Vue 3.
    <br />
    <a href="https://github.com/tuthanhh/maiscope/issues/new?labels=bug">Report Bug</a>
    &middot;
    <a href="https://github.com/tuthanhh/maiscope/issues/new?labels=enhancement">Request Feature</a>
  </p>
</div>



<!-- TABLE OF CONTENTS -->
<details>
  <summary>Table of Contents</summary>
  <ol>
    <li><a href="#about-the-project">About The Project</a></li>
    <li><a href="#built-with">Built With</a></li>
    <li>
      <a href="#getting-started">Getting Started</a>
      <ul>
        <li><a href="#prerequisites">Prerequisites</a></li>
        <li><a href="#installation">Installation</a></li>
      </ul>
    </li>
    <li><a href="#usage">Usage</a></li>
    <li><a href="#roadmap">Roadmap</a></li>
    <li><a href="#license">License</a></li>
    <li><a href="#acknowledgments">Acknowledgments</a></li>
  </ol>
</details>



<!-- ABOUT THE PROJECT -->
## About The Project

This is the frontend of **maiscope**, a song and chart browser for [maimai](https://maimai.sega.jp/) (SEGA's arcade rhythm game). It loads community-maintained song data from the maiscope backend and lets you browse, filter, and inspect charts across difficulties.

It is a port of [zetaraku/arcade-songs](https://github.com/zetaraku/arcade-songs) — the original web app, originally Nuxt 2 + Vuetify 2 — rebuilt on native Vue 3 for maimai only. This package also hosts the chart visualizer, a Bevy-engine-compiled-to-wasm player; see the repo root [README](../../README.md) for how it and the Rust backend fit into the rest of the monorepo.

**Features:**
- Song gallery with search and rich filtering (level, category, version, BPM, region, note designer)
- Table and grid views of the filtered results
- Per-sheet chart details (difficulty, internal level, note designer, note counts)
- Chart visualizer: plays back a sheet's chart, deep-linked from any sheet's detail view
- Build a working set of sheets ("My List") and filter down to just those
- Light / dark mode, multi-language UI (EN, JA, KO, ZH-Hans, ZH-Hant, VI, ES, ID, RU)




## Built With

[![Vue][Vue-badge]][Vue-url]
[![Vite][Vite-badge]][Vite-url]

| Package | Role |
|---|---|
| [`vue`](https://vuejs.org) 3 | UI framework |
| [`pinia`](https://pinia.vuejs.org) | State management |
| [`vue-router`](https://router.vuejs.org) | Routing |
| [`vue-i18n`](https://vue-i18n.intlify.dev) | Internationalization |
| [`@vueuse/core`](https://vueuse.org) | Composition utilities |
| [`yaml`](https://eemeli.org/yaml/) | Locale file parsing |
| [`vite`](https://vitejs.dev) 6 | Build tooling |




<!-- GETTING STARTED -->
## Getting Started

### Prerequisites

- [Node.js](https://nodejs.org/) 18+ and [pnpm](https://pnpm.io/) (pinned to `pnpm@10.33.0` via `packageManager`)
- [Rust toolchain](https://rustup.rs/) (stable) — this package's own code is plain TypeScript/Vue, but `pnpm build` and the visualizer route both need the chart engine's wasm bindings as a build input; see Usage

### Installation

1. Clone the monorepo
   ```sh
   git clone https://github.com/tuthanhh/maiscope.git
   cd maiscope/apps/host
   ```

2. Install dependencies
   ```sh
   pnpm install
   ```

3. Build the wasm chart engine (once, and again whenever `engine/` changes)
   ```sh
   ../../scripts/build-wasm.sh
   ```
   Output goes to `src/wasm/` (gitignored). `useEngine.ts` imports from this
   path, so both `pnpm dev`'s visualizer route and `pnpm build` need it —
   skipping this step leaves the rest of the app working but the visualizer
   and the production build broken.

4. Run the dev server
   ```sh
   pnpm dev
   ```




<!-- USAGE -->
## Usage

Launch the app and browse the song gallery. Use the filter panel to narrow by level, category, version, BPM, region, and note designer; open any song to inspect its charts, switch between table and grid views, or build a "My List" selection.

Song data is fetched from the maiscope backend API. The base URL is read from `VITE_API_BASE_URL` at build time (see `src/app/game.ts` and `.github/workflows/deploy-web.yml`); it defaults to `http://localhost:3000/api/v1` for local development against `apps/server`.

To produce a production build, rebuild the wasm bindings first (`release` is the size-optimised variant used for deploys — see `.github/workflows/deploy-web.yml`), then build:
```sh
../../scripts/build-wasm.sh release
pnpm build
```

This is a personal project and is not open for contributions at this time.




<!-- ROADMAP -->
## Roadmap

See [docs/ROADMAP.md](../../docs/ROADMAP.md) for the current milestones and what is deferred.




<!-- LICENSE -->
## License

Distributed under the MIT License.




<!-- ACKNOWLEDGMENTS -->
## Acknowledgments

- **[zetaraku/arcade-songs](https://github.com/zetaraku/arcade-songs)** by [zetaraku (Raku Zeta)](https://github.com/zetaraku) — the original web app this project is built on. All credit for the data model, filtering logic, and UI design goes to the upstream project.
- [maimai でらっくす 公式サイト｜セガ](https://maimai.sega.jp/) — official song information
- The maimai fan community — chart data and metadata




<!-- MARKDOWN LINKS & IMAGES -->
[contributors-shield]: https://img.shields.io/github/contributors/tuthanhh/maiscope.svg?style=for-the-badge
[contributors-url]: https://github.com/tuthanhh/maiscope/graphs/contributors
[forks-shield]: https://img.shields.io/github/forks/tuthanhh/maiscope.svg?style=for-the-badge
[forks-url]: https://github.com/tuthanhh/maiscope/network/members
[stars-shield]: https://img.shields.io/github/stars/tuthanhh/maiscope.svg?style=for-the-badge
[stars-url]: https://github.com/tuthanhh/maiscope/stargazers
[issues-shield]: https://img.shields.io/github/issues/tuthanhh/maiscope.svg?style=for-the-badge
[issues-url]: https://github.com/tuthanhh/maiscope/issues
[license-shield]: https://img.shields.io/github/license/tuthanhh/maiscope.svg?style=for-the-badge
[license-url]: https://github.com/tuthanhh/maiscope/blob/master/LICENSE
[Vue-badge]: https://img.shields.io/badge/Vue.js-35495E?style=for-the-badge&logo=vuedotjs&logoColor=4FC08D
[Vue-url]: https://vuejs.org/
[Vite-badge]: https://img.shields.io/badge/Vite-646CFF?style=for-the-badge&logo=vite&logoColor=white
[Vite-url]: https://vitejs.dev/
