use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::Context;
use clap::Parser;
use color_print::cprintln;
use copy_dir::copy_dir;
use glob::glob;
use mcvm_core::launch::LaunchConfiguration;
use mcvm_core::net::download;
use mcvm_core::util::versions::MinecraftVersion;
use mcvm_core::{ConfigBuilder, InstanceConfiguration, InstanceKind, MCVMCore};
use mcvm_mods::fabric_quilt;
use mcvm_shared::{output, Side};

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
    let mut cli = Cli::parse();

    let minecraft_version = if let Some(minecraft_version) = cli.minecraft_version {
        minecraft_version
    } else {
        "1.20.4".into()
    };

    cli.packtest_url = cli.packtest_url.filter(|x| x != "latest");
    let packtest_url = if let Some(packtest_url) = cli.packtest_url {
        packtest_url
    } else {
        get_packtest_url(&minecraft_version)
            .context("No PackTest available for this Minecraft version")?
            .into()
    };

    cli.fabric_api_url = cli.fabric_api_url.filter(|x| x != "latest");
    let fabric_api_url = if let Some(fabric_api_url) = cli.fabric_api_url {
        fabric_api_url
    } else {
        get_fabric_api_url(&minecraft_version)
            .context("No Fabric API available for this Minecraft version")?
            .into()
    };

    let mut o = output::Simple(output::MessageLevel::Trace);
    if cli.github {
        println!("::group::Install test server::")
    }
    let core_config = ConfigBuilder::new().disable_hardlinks(true);
    let mut core = MCVMCore::with_config(core_config.build()).context("Failed to create core")?;
    let version_info = core
        .get_version_info(minecraft_version.clone())
        .await
        .context("Failed to get Minecraft version info")?;

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
        .get_version(&MinecraftVersion::Version(minecraft_version.into()), &mut o)
        .await
        .context("Failed to create Minecraft version")?;

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
        packtest_url,
        &mods_dir.join("packtest.jar"),
        &reqwest::Client::new(),
    )
    .await
    .context("Failed to download Packtest mod")?;
    download::file(
        fabric_api_url,
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
    for pack_pattern in packs {
        for pack in glob(&pack_pattern).context(format!(
            "Failed to parse glob pattern for pattern {pack_pattern}"
        ))? {
            let pack = pack?;
            println!("Copying pack {}", pack.to_string_lossy());
            let in_path = PathBuf::from(pack.clone());
            let out_path = datapack_dir.join(in_path.file_name().context("Missing filename")?);
            copy_dir(in_path, out_path).context(format!(
                "Failed to copy pack {} into world",
                pack.to_string_lossy()
            ))?;
        }
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
    let handle = instance
        .launch_with_handle(&mut o)
        .await
        .context("Failed to launch instance")?;
    if cli.github {
        println!("::endgroup::")
    }

    let mut process = handle.get_process();
    let status = process.wait().context("Failed to await process")?;

    // Check for test failure
    if cli.github {
        println!("::group::Get result::")
    }
    if !status.success() {
        print_fail_message("Exit code was non-zero");
        return Ok(true);
    }

    let log = std::fs::read_to_string(inst_dir.join("logs/latest.log"))
        .context("Failed to open log file")?;
    if log.contains("required tests failed") {
        print_fail_message("Required tests failed");
        return Ok(true);
    }
    if log.contains("Failed to load test") {
        print_fail_message("A test failed to load");
        return Ok(true);
    }
    if log.contains("All 0 required tests") {
        print_fail_message("No tests were found");
        return Ok(true);
    }

    cprintln!("<g>Test run successful :D");
    if cli.github {
        println!("::endgroup::")
    }

    Ok(false)
}

fn print_fail_message(msg: &str) {
    cprintln!("<r>Tests failed because:\n  {msg}");
}

fn get_packtest_url(version: &str) -> Option<&'static str> {
    match version {
        "1.20.4" => Some(
            "https://github.com/misode/packtest/releases/download/v1.3/packtest-1.3-mc1.20.4.jar",
        ),
        _ => None,
    }
}

fn get_fabric_api_url(version: &str) -> Option<&'static str> {
    match version {
        "1.20.4" => Some("https://cdn.modrinth.com/data/P7dR8mSH/versions/JQ07mKWY/fabric-api-0.91.3%2B1.20.4.jar"),
        _ => None,
    }
}

#[derive(Parser)]
struct Cli {
    /// If set, then parses the first pack specified as a list of packs separated by commas
    #[arg(long)]
    comma_separate: bool,
    /// Whether to output special messages for use in GitHub Actions
    #[arg(long)]
    github: bool,
    /// Minecraft version to use. Defaults to 1.20.4. Will determine
    /// the PackTest and Fabric API URLs to use unless one is overridden
    #[arg(short, long)]
    minecraft_version: Option<String>,
    /// URL to the PackTest mod to use. Defaults to URL for the latest version
    /// for the specified Minecraft version
    #[arg(long)]
    packtest_url: Option<String>,
    /// URL to the Fabric API mod to use. Defaults to URL for the latest version
    /// for the specified Minecraft version
    #[arg(long)]
    fabric_api_url: Option<String>,
    /// The packs to test. They must all be datapacks with the mcmeta
    /// in the root directory
    packs: Vec<String>,
}
