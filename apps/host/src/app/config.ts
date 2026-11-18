export interface AppConfig {
  siteTitle: string;
  siteReportUrl: string;
  sourceCodeUrl: string;
}

// The menu's external links and About point here: this project's repository,
// not the upstream arcade-songs it was ported from (About credits that).
const config: AppConfig = {
  siteTitle: "maiscope",
  siteReportUrl: "https://github.com/tuthanhh/maiscope/issues",
  sourceCodeUrl: "https://github.com/tuthanhh/maiscope",
};

export default config;
