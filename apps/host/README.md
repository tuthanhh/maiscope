[![Contributors][contributors-shield]][contributors-url]
[![Forks][forks-shield]][forks-url]
[![Stargazers][stars-shield]][stars-url]
[![Issues][issues-shield]][issues-url]
[![MIT License][license-shield]][license-url]



<!-- PROJECT LOGO -->
<br />
<div align="center">
<h3 align="center">maiscope-frontend</h3>

  <p align="center">
    A desktop song &amp; chart browser for arcade rhythm games, built with Vue 3 and Tauri.
    <br />
    <a href="https://github.com/tuthanhh/maiscope-frontend/issues/new?labels=bug">Report Bug</a>
    &middot;
    <a href="https://github.com/tuthanhh/maiscope-frontend/issues/new?labels=enhancement">Request Feature</a>
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

**maiscope-frontend** is a desktop song and chart browser for arcade rhythm games such as [maimai](https://maimai.sega.jp/). It loads community-maintained song data and lets you browse, filter, and inspect charts across difficulties.

It is a desktop port of [zetaraku/arcade-songs](https://github.com/zetaraku/arcade-songs) — the original web app — wrapped in [Tauri](https://tauri.app/) for a native window and bundled distribution.

**Features:**
- Song gallery with cover art, search, and rich filtering (level, genre, version, BPM, …)
- Per-sheet chart details (difficulty, internal level, designer, note counts)
- Data table, grid, and chart (echarts) views
- Timeline view of song / version history
- Light / dark mode, multi-language UI (EN, JA, KO, ZH, VI, ES, ID, RU)
- My List export and shareable filter state via URL query




## Built With

[![Vue][Vue-badge]][Vue-url]
[![Vuetify][Vuetify-badge]][Vuetify-url]
[![Tauri][Tauri-badge]][Tauri-url]
[![Vite][Vite-badge]][Vite-url]

| Package | Role |
|---|---|
| [`vue`](https://vuejs.org) 3 | UI framework |
| [`vuetify`](https://vuetifyjs.com) 3 | Material component library |
| [`pinia`](https://pinia.vuejs.org) | State management |
| [`vue-router`](https://router.vuejs.org) | Routing |
| [`vue-i18n`](https://vue-i18n.intlify.dev) | Internationalization |
| [`echarts`](https://echarts.apache.org) + [`vue-echarts`](https://github.com/ecomfe/vue-echarts) | Data visualization |
| [`@tauri-apps/api`](https://tauri.app) 2 | Native desktop shell |
| [`vite`](https://vitejs.dev) 6 | Build tooling |




<!-- GETTING STARTED -->
## Getting Started

### Prerequisites

- [Node.js](https://nodejs.org/) 18+ and [pnpm](https://pnpm.io/)
- [Rust toolchain](https://rustup.rs/) (stable) and the [Tauri system prerequisites](https://tauri.app/start/prerequisites/) for your OS

### Installation

1. Clone the repo
   ```sh
   git clone https://github.com/tuthanhh/maiscope-frontend.git
   cd maiscope-frontend
   ```

2. Install dependencies
   ```sh
   pnpm install
   ```

3. Run in the browser (Vite dev server)
   ```sh
   pnpm dev
   ```

4. Run as a desktop app (Tauri)
   ```sh
   pnpm tauri dev
   ```




<!-- USAGE -->
## Usage

Launch the app, pick a game, and browse the song gallery. Use the filter panel to narrow by level, genre, version, BPM, and more; open any song to inspect its charts, switch between table / grid / chart views, or build and export a My List.

Song data is fetched from the community data source configured in `src/data/sites.json`.

To build a distributable desktop binary:
```sh
pnpm tauri build
```

This is a personal project and is not open for contributions at this time.




<!-- ROADMAP -->
## Roadmap

Moving off the CloudFront `data.json` feed onto a backend API, with an offline-first local cache, desktop contribution flow, and chart visualization.

- [ ] **C1** — Point client at backend read API (`GET /api/v1/games/{code}/data`), replacing CloudFront; keep current behavior
- [ ] **C2** — Local SQLite cache (`tauri-plugin-sql`) + offline-first + delta sync via manifest
- [ ] **C3** — GitHub login + contribution UI (submit chart, my contributions)
- [ ] **C4** — Chart / audio download + maimai visualization engine

See [open issues](https://github.com/tuthanhh/maiscope-frontend/issues) for tracked items.




<!-- LICENSE -->
## License

Distributed under the MIT License.




<!-- ACKNOWLEDGMENTS -->
## Acknowledgments

- **[zetaraku/arcade-songs](https://github.com/zetaraku/arcade-songs)** by [zetaraku (Raku Zeta)](https://github.com/zetaraku) — the original web app this project is built on. All credit for the data model, filtering logic, and UI design goes to the upstream project.
- [maimai でらっくす 公式サイト｜セガ](https://maimai.sega.jp/) — official song information
- The maimai fan community — chart data and metadata




<!-- MARKDOWN LINKS & IMAGES -->
[contributors-shield]: https://img.shields.io/github/contributors/tuthanhh/maiscope-frontend.svg?style=for-the-badge
[contributors-url]: https://github.com/tuthanhh/maiscope-frontend/graphs/contributors
[forks-shield]: https://img.shields.io/github/forks/tuthanhh/maiscope-frontend.svg?style=for-the-badge
[forks-url]: https://github.com/tuthanhh/maiscope-frontend/network/members
[stars-shield]: https://img.shields.io/github/stars/tuthanhh/maiscope-frontend.svg?style=for-the-badge
[stars-url]: https://github.com/tuthanhh/maiscope-frontend/stargazers
[issues-shield]: https://img.shields.io/github/issues/tuthanhh/maiscope-frontend.svg?style=for-the-badge
[issues-url]: https://github.com/tuthanhh/maiscope-frontend/issues
[license-shield]: https://img.shields.io/github/license/tuthanhh/maiscope-frontend.svg?style=for-the-badge
[license-url]: https://github.com/tuthanhh/maiscope-frontend/blob/master/LICENSE
[Vue-badge]: https://img.shields.io/badge/Vue.js-35495E?style=for-the-badge&logo=vuedotjs&logoColor=4FC08D
[Vue-url]: https://vuejs.org/
[Vuetify-badge]: https://img.shields.io/badge/Vuetify-1867C0?style=for-the-badge&logo=vuetify&logoColor=white
[Vuetify-url]: https://vuetifyjs.com/
[Tauri-badge]: https://img.shields.io/badge/Tauri-24C8DB?style=for-the-badge&logo=tauri&logoColor=white
[Tauri-url]: https://tauri.app/
[Vite-badge]: https://img.shields.io/badge/Vite-646CFF?style=for-the-badge&logo=vite&logoColor=white
[Vite-url]: https://vitejs.dev/
