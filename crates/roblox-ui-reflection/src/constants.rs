use const_format::concatcp;

pub const URL_VERSION: &str = "https://setup.rbxcdn.com/versionQTStudio";
pub const URL_VERSION_MARKER: &str = "<<VERSION>>";

pub const URL_STUDIO: &str = concatcp!(
    "https://setup.rbxcdn.com/",
    URL_VERSION_MARKER,
    "-RobloxStudio.zip"
);
