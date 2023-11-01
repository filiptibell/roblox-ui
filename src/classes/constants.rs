use const_format::concatcp;

pub(super) const URL_REPO: &str = "MaximumADHD/Roblox-Client-Tracker";
pub(super) const URL_BRANCH: &str = "roblox";
pub(super) const URL_BASE: &str = concatcp!(
    "https://raw.githubusercontent.com/",
    URL_REPO,
    "/",
    URL_BRANCH
);

pub(super) const API_DOCS_URL: &str = concatcp!(URL_BASE, "/api-docs/mini/en-us.json");
