export interface AppConfig {
  siteTitle: string;
  siteUrl: string;
  siteDescriptionEn: string;
  siteDescriptionJp: string;
  siteReportUrl: string;
  sourceCodeUrl: string;
  indexAccessCounterUrl?: string;
  [key: string]: unknown;
}

const config: AppConfig = {
  siteTitle: "maiscope",
  siteUrl: "https://arcade-songs.zetaraku.dev/",
  siteDescriptionEn: "A song browser of various arcade games including ______.",
  siteDescriptionJp: "______を含む様々なアーケードゲームの楽曲ブラウザ。",
  siteReportUrl: "https://github.com/zetaraku/arcade-songs/issues",
  sourceCodeUrl: "https://github.com/zetaraku/arcade-songs",
  indexAccessCounterUrl: undefined,
};

export default config;
