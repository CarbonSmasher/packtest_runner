use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::Context;
use clap::Parser;
use mcvm_core::launch::LaunchConfiguration;
use mcvm_core::net::download;
use mcvm_core::util::versions::MinecraftVersion;
use mcvm_core::{InstanceConfiguration, InstanceKind, MCVMCore};
use mcvm_mods::fabric_quilt;
use mcvm_shared::{output, Side};

const PACKTEST_URL: &str =
    "https://github.com/misode/packtest/releases/download/v1.0.0-beta.2/packtest-1.0.0-beta.2.jar";
const FABRIC_API_URL: &str =
    "https://cdn.modrinth.com/data/P7dR8mSH/versions/JQ07mKWY/fabric-api-0.91.3%2B1.20.4.jar";

const SERVER_PROPERTIES: &str = "
rcon.port=25575
online-mode=false
broadcast-rcon-to-ops=true
enable-rcon=false
rcon.password=packtest
";

#[tokio::main]
async fn main() -> ExitCode {
    let status = run().await.expect("Failed to run");
    if status {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

async fn run() -> anyhow::Result<bool> {
    let cli = Cli::parse();

    let version = "1.20.4";
    let mut o = output::Simple(output::MessageLevel::Trace);
    let mut core = MCVMCore::new().context("Failed to create core")?;
    let version_info = core
        .get_version_info(version.into())
        .await
        .context("Failed to get version info")?;

    let (classpath, main_class) = fabric_quilt::install_from_core(
        &mut core,
        &version_info,
        fabric_quilt::Mode::Fabric,
        Side::Server,
        &mut o,
    )
    .await
    .context("Failed to install Fabric/Quilt")?;

    let mut vers = core
        .get_version(&MinecraftVersion::Version(version.into()), &mut o)
        .await
        .context("Failed to create version")?;

    let inst_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("packtest_launch");
    let mut launch_config = LaunchConfiguration::new();
    launch_config.jvm_args = vec!["-Dpacktest.auto".into()];
    let inst_config = InstanceConfiguration {
        side: InstanceKind::Server {
            create_eula: true,
            show_gui: false,
        },
        path: inst_dir.clone(),
        launch: launch_config,
        jar_path: None,
        main_class: Some(main_class),
        additional_libs: classpath.get_paths(),
    };
    let mut instance = vers
        .get_instance(inst_config, &mut o)
        .await
        .context("Failed to create instance")?;

    let mods_dir = inst_dir.join("mods");
    if !mods_dir.exists() {
        std::fs::create_dir(&mods_dir)?;
    }
    // Download the mods
    download::file(
        FABRIC_API_URL,
        &mods_dir.join("packtest.jar"),
        &reqwest::Client::new(),
    )
    .await
    .context("Failed to download Packtest mod")?;
    download::file(
        PACKTEST_URL,
        &mods_dir.join("fabric_api.jar"),
        &reqwest::Client::new(),
    )
    .await
    .context("Failed to download Fabric API mod")?;

    // Move the datapacks into the world
    let datapack_dir = inst_dir.join("world").join("datapacks");
    std::fs::create_dir_all(&datapack_dir).context("Failed to create world datapacks directory")?;
    for pack in &cli.packs {
        let in_path = PathBuf::from(pack);
        let out_path = datapack_dir.join(in_path.file_name().context("Missing filename")?);

        std::fs::copy(in_path, out_path).context("Failed to copy pack into world")?;
    }

    // Create the server properties
    std::fs::write(inst_dir.join("server.properties"), SERVER_PROPERTIES)
        .context("Failed to write server properties")?;

    instance
        .launch(&mut o)
        .await
        .context("Failed to launch instance")?;

    // Check for test failure
    let log = std::fs::read_to_string(inst_dir.join("logs/latest.log"))
        .context("Failed to open log file")?;
    let failed = log.contains("tests failed");

    Ok(failed)
}

#[derive(Parser)]
struct Cli {
    /// The packs to test. They must all be zip files with the mcmeta
    /// in the root directory
    packs: Vec<String>,
}
