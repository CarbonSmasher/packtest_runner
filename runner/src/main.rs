use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::Context;
use clap::Parser;
use copy_dir::copy_dir;
use mcvm_core::launch::LaunchConfiguration;
use mcvm_core::net::download;
use mcvm_core::util::versions::MinecraftVersion;
use mcvm_core::{ConfigBuilder, InstanceConfiguration, InstanceKind, MCVMCore};
use mcvm_mods::fabric_quilt;
use mcvm_shared::{output, Side};

const PACKTEST_URL: &str =
    "https://github.com/misode/packtest/releases/download/v1.0.0-beta4/packtest-1.0.0-beta4.jar";
const FABRIC_API_URL: &str =
    "https://cdn.modrinth.com/data/P7dR8mSH/versions/JQ07mKWY/fabric-api-0.91.3%2B1.20.4.jar";

const SERVER_PROPERTIES: &str = "
rcon.port=25575
online-mode=false
broadcast-rcon-to-ops=true
enable-rcon=false
rcon.password=packtest
level-type=minecraft\\:flat
snooper-enabled=false
generate-structures=false
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

    let version = if let Some(version) = cli.version {
        version
    } else {
        "1.20.4".into()
    };

    let mut o = output::Simple(output::MessageLevel::Trace);
    if cli.github {
        println!("::group::Install test server::")
    }
    let core_config = ConfigBuilder::new().disable_hardlinks(true);
    let mut core = MCVMCore::with_config(core_config.build()).context("Failed to create core")?;
    let version_info = core
        .get_version_info(version.clone())
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

    let inst_dir = PathBuf::from("./packtest_launch");
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

    let packs = if cli.comma_separate {
        cli.packs
            .first()
            .context("Missing first pack to split")?
            .split(',')
            .map(|x| x.to_string())
            .collect::<Vec<_>>()
    } else {
        cli.packs
    };
    for pack in packs {
        println!("Copying pack {pack}");
        let in_path = PathBuf::from(pack.clone());
        let out_path = datapack_dir.join(in_path.file_name().context("Missing filename")?);
        copy_dir(in_path, out_path).context(format!("Failed to copy pack {pack} into world"))?;
    }

    // Create the server properties
    std::fs::write(inst_dir.join("server.properties"), SERVER_PROPERTIES)
        .context("Failed to write server properties")?;

    if cli.github {
        println!("::endgroup::")
    }

    if cli.github {
        println!("::group::Launch server and run tests::")
    }
    instance
        .launch(&mut o)
        .await
        .context("Failed to launch instance")?;
    if cli.github {
        println!("::endgroup::")
    }

    // Check for test failure
    let log = std::fs::read_to_string(inst_dir.join("logs/latest.log"))
        .context("Failed to open log file")?;
    let failed = log.contains("tests failed");

    Ok(failed)
}

#[derive(Parser)]
struct Cli {
    /// If set, then parses the first pack specified as a list of packs separated by commas
    #[arg(long)]
    comma_separate: bool,

    /// Whether to output special messages for use in GitHub Actions
    #[arg(long)]
    github: bool,

    /// Minecraft version to use. Defaults to 1.20.4
    #[arg(short, long)]
    version: Option<String>,

    /// The packs to test. They must all be datapacks with the mcmeta
    /// in the root directory
    packs: Vec<String>,
}
