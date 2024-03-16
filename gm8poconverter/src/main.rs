use gm8exe::{
    asset::{CodeAction, Extension, Font, IncludedFile, Object, PascalString, Script},
    GameAssets, GameVersion,
};
use rayon::vec;
use std::{
    collections::HashMap,
    env, fs,
    path::{Path, PathBuf},
    process,
};

pub mod collision;
pub mod deobfuscate;
pub mod gmk;
pub mod mappings;
pub mod zlib;

static INFO_STRING: &str = concat!(
    "GM8PoConverter v",
    env!("CARGO_PKG_VERSION"),
    " for ",
    env!("TARGET_TRIPLE"),
    " - built on ",
    env!("BUILD_DATE"),
    ", #",
    env!("GIT_HASH"),
);

// Know to "press any key" but only if double-clicked in WinExplorer or whatever.
#[cfg(windows)]
fn is_cmd(argv_0: &str) -> bool {
    let is_argv0_absolute = Path::new(argv_0).is_absolute();
    let is_msys2 = env::var("MSYSTEM").is_ok();

    is_argv0_absolute && !is_msys2
}
#[cfg(windows)]
fn pause(tip: bool) {
    extern "C" {
        fn _getch() -> std::os::raw::c_int;
    }
    if tip {
        println!("\nTip: To convert a game, click and drag it on top of the executable.");
    }
    println!("<< Press Any Key >>");
    let _ = unsafe { _getch() };
}
#[cfg(not(windows))]
fn is_cmd(_argv_0: &str) -> bool {
    false
}
#[cfg(not(windows))]
fn pause(_tip: bool) {}

fn main() {
    println!("{}", INFO_STRING);

    let args: Vec<String> = env::args().collect();
    assert!(!args.is_empty());
    let process_path = args[0].as_str();
    let should_pause = is_cmd(process_path);

    // set up getopts to parse our command line args
    let mut opts = getopts::Options::new();
    opts.optflag("h", "help", "print this help message")
        .optflag("l", "lazy", "disable various data integrity checks")
        .optflag("v", "verbose", "enable verbose logging for decompilation")
        .optopt("d", "deobfuscate", "set deobfuscation mode auto/on/off (default=auto)", "")
        .optflag("p", "preserve", "preserve broken events (instead of trying to fix them)")
        .optflag("s", "singlethread", "decompile gamedata synchronously (lower RAM usage)")
        .optopt("o", "output", "specify output filename", "FILE");

    // parse command line arguments
    let matches = match opts.parse(&args[1..]) {
        Ok(matches) => matches,
        Err(err) => {
            use getopts::Fail::*;
            match err {
                ArgumentMissing(arg) => eprintln!("Missing argument: {}", arg),
                UnrecognizedOption(opt) => eprintln!("Unrecognized option: {}", opt),
                OptionMissing(opt) => eprintln!("Missing option: {}", opt),
                OptionDuplicated(opt) => eprintln!("Duplicated option: {}", opt),
                UnexpectedArgument(arg) => eprintln!("Unexpected argument: {}", arg),
            }
            if should_pause {
                pause(true);
            }
            process::exit(1);
        },
    };

    // print help message if requested OR no input files
    if matches.opt_present("h") || matches.free.is_empty() {
        // If the getopts2 usage generator didn't suck this much,
        // I wouldn't have to resort to this.
        // TODO: Get a better argument parser in general.
        println!(
            "Usage: {} FILENAME [options]

Options:
    -h, --help                print this help message
    -l, --lazy                disable various data integrity checks
    -v, --verbose             enable verbose logging for decompilation
    -d, --deobfuscate <mode>  set deobfuscation mode auto/on/off (defaults to auto)
    -p, --preserve            preserve broken events (instead of trying to fix them)
    -s, --singlethread        decompile gamedata synchronously (lower RAM usage)
    -o, --output <file>       specify output filename",
            process_path
        );
        if should_pause {
            pause(true);
        }
        process::exit(0); // once the user RTFM they can run it again
    }

    // print error message if multiple inputs were provided
    if matches.free.len() > 1 {
        eprintln!(
            concat!("Unexpected input: {}\n", "Tip: Only one input gamefile is expected at a time!",),
            matches.free[1]
        );
        if should_pause {
            pause(true);
        }
        process::exit(1);
    }

    // extract flags & input path
    let input = &matches.free[0];
    let lazy = matches.opt_present("l");
    let singlethread = matches.opt_present("s");
    let verbose = matches.opt_present("v");
    let deobfuscate = match matches.opt_str("d").as_deref() {
        Some("on") => deobfuscate::Mode::On,
        Some("off") => deobfuscate::Mode::Off,
        Some("auto") | None => deobfuscate::Mode::Auto,
        Some(x) => {
            eprintln!("Invalid deobfuscator setting: {} (valid settings are on/off/auto)", x);
            process::exit(1);
        },
    };
    let out_path = matches.opt_str("o");
    let preserve = matches.opt_present("p");
    // no_pause extracted before help

    // print flags for confirmation
    println!("Input file: {}", input);
    if lazy {
        println!("Lazy mode ON: data integrity checking disabled");
    }
    if verbose {
        println!("Verbose logging ON: verbose console output enabled");
    }
    match deobfuscate {
        deobfuscate::Mode::On => println!("Deobfuscation ON: will standardise GML code"),
        deobfuscate::Mode::Off => println!("Deobfuscation OFF: will ignore obfuscation"),
        _ => (),
    }
    if singlethread {
        println!("Single-threaded mode ON: process will not start new threads (slow)");
    }
    if let Some(path) = &out_path {
        println!("Specified output path: {}", path);
    }
    if preserve {
        println!("Preserve mode ON: broken events will be preserved and will not be fixed");
    }

    // resolve input path
    let input_path = Path::new(input);
    if !input_path.is_file() {
        eprintln!("Input file '{}' does not exist.", input);
        process::exit(1);
    }

    // allow decompile to handle the rest of main
    if let Err(e) = decompile(input_path, out_path, !lazy, !singlethread, verbose, deobfuscate, !preserve) {
        eprintln!("Error parsing gamedata:\n{}", e);
        process::exit(1);
    }

    if should_pause {
        pause(false);
    }
}

fn decompile(
    in_path: &Path,
    out_path: Option<String>,
    strict: bool,
    multithread: bool,
    verbose: bool,
    deobf_mode: deobfuscate::Mode,
    fix_events: bool,
) -> Result<(), String> {
    // slurp in file contents
    let file = fs::read(&in_path).map_err(|e| format!("Failed to read '{}': {}", in_path.display(), e))?;

    // parse (entire) gamedata
    let logger = if verbose { Some(|msg: &str| println!("{}", msg)) } else { None };
    let mut assets = gm8exe::reader::from_exe(file, logger, strict, multithread) // huge call
        .map_err(|e| format!("Reader error: {}", e))?;

    println!("Successfully parsed game!");

    //Do we want to deobfuscate, yes or no?
    let deobfuscate = match deobf_mode {
        deobfuscate::Mode::On => true,
        deobfuscate::Mode::Off => false,
        deobfuscate::Mode::Auto => {
            assets.backgrounds.iter().flatten().any(|s| s.name.0.is_empty())
                || assets.fonts.iter().flatten().any(|s| s.name.0.is_empty())
                || assets.objects.iter().flatten().any(|s| s.name.0.is_empty())
                || assets.paths.iter().flatten().any(|s| s.name.0.is_empty())
                || assets.rooms.iter().flatten().any(|s| s.name.0.is_empty())
                || assets.sounds.iter().flatten().any(|s| s.name.0.is_empty())
                || assets.sprites.iter().flatten().any(|s| s.name.0.is_empty())
                || assets.timelines.iter().flatten().any(|s| s.name.0.is_empty())
        },
    };
    if deobf_mode == deobfuscate::Mode::Auto && deobfuscate {
        println!("Note: GMK looks obfuscated, so de-obfuscation has been enabled by default");
        println!(" -- you can turn this off with '-d off'");
    }

    fn fix_event(ev: &mut gm8exe::asset::CodeAction) {
        // So far the only broken event type I know of is custom Execute Code actions.
        // We can fix these by changing the act id and lib id to be a default Execute Code action instead.
        if ev.action_kind == 7 && ev.execution_type == 2 {
            // 7 = code block param, 2 = code execution
            ev.id = 603;
            ev.lib_id = 1;
        }
    }

    if fix_events {
        assets
            .objects
            .iter_mut()
            .flatten()
            .flat_map(|x| x.events.iter_mut().flatten())
            .flat_map(|(_, x)| x.iter_mut())
            .for_each(|ev| fix_event(ev));

        assets
            .timelines
            .iter_mut()
            .flatten()
            .flat_map(|x| x.moments.iter_mut().flat_map(|(_, x)| x.iter_mut()))
            .for_each(|ev| fix_event(ev));
    }

    // warn user if they specified .gmk for 8.0 or .gm81 for 8.0
    let out_expected_ext = match assets.version {
        GameVersion::GameMaker8_0 => "gmk",
        GameVersion::GameMaker8_1 => "gm81",
    };
    let out_path = match out_path {
        Some(p) => {
            let path = PathBuf::from(p);
            match (assets.version, path.extension().and_then(|oss| oss.to_str())) {
                (GameVersion::GameMaker8_0, Some(extension @ "gm81"))
                | (GameVersion::GameMaker8_1, Some(extension @ "gmk")) => {
                    println!(
                        concat!(
                            "***WARNING*** You've specified an output file '{}'",
                            "a .{} file, for a {} game.\nYou should use '-o {}.{}' instead, ",
                            "otherwise you won't be able to load the file with GameMaker.",
                        ),
                        path.display(),
                        extension,
                        match assets.version {
                            GameVersion::GameMaker8_0 => "GameMaker 8.0",
                            GameVersion::GameMaker8_1 => "GameMaker 8.1",
                        },
                        path.file_stem().and_then(|oss| oss.to_str()).unwrap_or("filename"),
                        out_expected_ext,
                    );
                },
                _ => (),
            }
            path
        },
        None => {
            let mut path = PathBuf::from(in_path);
            path.set_extension(out_expected_ext);
            path
        },
    };

    if deobfuscate {
        deobfuscate::process(&mut assets);
    }

    patch(&mut assets, in_path);

    let mut gmk = fs::File::create(&out_path)
        .map_err(|e| format!("Failed to create output file '{}': {}", out_path.display(), e))?;

    println!("Writing {} header...", out_expected_ext);
    gmk::write_header(&mut gmk, assets.version, assets.game_id, assets.guid)
        .map_err(|e| format!("Failed to write header: {}", e))?;

    println!("Writing {} settings...", out_expected_ext);
    let ico_file = assets.ico_file_raw.take();
    gmk::write_settings(&mut gmk, &assets.settings, ico_file, assets.version)
        .map_err(|e| format!("Failed to write settings block: {}", e))?;

    println!("Writing {} triggers...", assets.triggers.len());
    gmk::write_asset_list(&mut gmk, &assets.triggers, gmk::write_trigger, assets.version, multithread)
        .map_err(|e| format!("Failed to write triggers: {}", e))?;

    gmk::write_timestamp(&mut gmk).map_err(|e| format!("Failed to write timestamp: {}", e))?;

    println!("Writing {} constants...", assets.constants.len());
    gmk::write_constants(&mut gmk, &assets.constants).map_err(|e| format!("Failed to write constants: {}", e))?;

    println!("Writing {} sounds...", assets.sounds.len());
    gmk::write_asset_list(&mut gmk, &assets.sounds, gmk::write_sound, assets.version, multithread)
        .map_err(|e| format!("Failed to write sounds: {}", e))?;

    println!("Writing {} sprites...", assets.sprites.len());
    gmk::write_asset_list(&mut gmk, &assets.sprites, gmk::write_sprite, assets.version, multithread)
        .map_err(|e| format!("Failed to write sprites: {}", e))?;

    println!("Writing {} backgrounds...", assets.backgrounds.len());
    gmk::write_asset_list(&mut gmk, &assets.backgrounds, gmk::write_background, assets.version, multithread)
        .map_err(|e| format!("Failed to write backgrounds: {}", e))?;

    println!("Writing {} paths...", assets.paths.len());
    gmk::write_asset_list(&mut gmk, &assets.paths, gmk::write_path, assets.version, multithread)
        .map_err(|e| format!("Failed to write paths: {}", e))?;

    println!("Writing {} scripts...", assets.scripts.len());
    gmk::write_asset_list(&mut gmk, &assets.scripts, gmk::write_script, assets.version, multithread)
        .map_err(|e| format!("Failed to write scripts: {}", e))?;

    println!("Writing {} fonts...", assets.fonts.len());
    gmk::write_asset_list(&mut gmk, &assets.fonts, gmk::write_font, assets.version, multithread)
        .map_err(|e| format!("Failed to write fonts: {}", e))?;

    println!("Writing {} timelines...", assets.timelines.len());
    gmk::write_asset_list(&mut gmk, &assets.timelines, gmk::write_timeline, assets.version, multithread)
        .map_err(|e| format!("Failed to write timelines: {}", e))?;

    println!("Writing {} objects...", assets.objects.len());
    gmk::write_asset_list(&mut gmk, &assets.objects, gmk::write_object, assets.version, multithread)
        .map_err(|e| format!("Failed to write objects: {}", e))?;

    println!("Writing {} rooms...", assets.rooms.len());
    gmk::write_asset_list(&mut gmk, &assets.rooms, gmk::write_room, assets.version, multithread)
        .map_err(|e| format!("Failed to write rooms: {}", e))?;

    println!(
        "Writing room editor metadata... (last instance: {}, last tile: {})",
        assets.last_instance_id, assets.last_tile_id
    );
    gmk::write_room_editor_meta(&mut gmk, assets.last_instance_id, assets.last_tile_id)
        .map_err(|e| format!("Failed to write room editor metadata: {}", e))?;

    println!("Writing {} included files...", assets.included_files.len());
    gmk::write_included_files(&mut gmk, &assets.included_files)
        .map_err(|e| format!("Failed to write included files: {}", e))?;

    println!("Writing {} extensions...", assets.extensions.len());
    gmk::write_extensions(&mut gmk, &assets.extensions).map_err(|e| format!("Failed to write extensions: {}", e))?;

    println!("Writing game information...");
    gmk::write_game_information(&mut gmk, &assets.help_dialog)
        .map_err(|e| format!("Failed to write game information: {}", e))?;

    println!("Writing {} library initialization strings...", assets.library_init_strings.len());
    gmk::write_library_init_code(&mut gmk, &assets.library_init_strings)
        .map_err(|e| format!("Failed to write library initialization code: {}", e))?;

    println!("Writing room order ({} rooms)...", assets.room_order.len());
    gmk::write_room_order(&mut gmk, &assets.room_order).map_err(|e| format!("Failed to write room order: {}", e))?;

    println!("Writing resource tree...");
    gmk::write_resource_tree(&mut gmk, &assets).map_err(|e| format!("Failed to write resource tree: {}", e))?;

    println!(
        "Successfully written {} to '{}'",
        out_expected_ext,
        out_path.file_name().and_then(|oss| oss.to_str()).unwrap_or("<INVALID UTF-8>"),
    );

    Ok(())
}

fn patch(assets: &mut GameAssets, in_path: &Path) {
    // Test engine
    enum Engine {
        Unknown,
        Renex,
        Verve,
    }

    let mut engine = Engine::Unknown;

    for scr in assets.scripts.iter() {
        if let Some(scr) = scr {
            engine = match scr.name.to_string().as_str() {
                "save_save" | "player_air_jump" => Engine::Verve,
                "custom_sound_properties" => Engine::Renex,
                _ => engine,
            };
        }
    }

    let data = fs::read(in_path).unwrap();
    let game_id = format!("{:x}", md5::compute(data));
    let game_name = in_path.file_stem().unwrap().to_str().unwrap();
    let server_ip = "81.70.53.71";

    match engine {
        Engine::Verve => {
            println!("Verve engine detected!");
            println!("Adding http dll scripts...");
            add_http_scripts(assets);

            println!("Adding online objects...");
            add_online_objects(
                assets,
                include_str!("./gml/verve/__ONLINE_onlinePlayer_Create.gml").into(),
                include_str!("./gml/verve/__ONLINE_onlinePlayer_EndStep.gml").into(),
                include_str!("./gml/verve/__ONLINE_onlinePlayer_Draw.gml").into(),
                include_str!("./gml/verve/__ONLINE_chatbox_Create.gml").into(),
                include_str!("./gml/verve/__ONLINE_chatbox_EndStep.gml").into(),
                include_str!("./gml/verve/__ONLINE_chatbox_Draw.gml").into(),
                include_str!("./gml/verve/__ONLINE_playerSaved_Draw.gml").into(),
                include_str!("./gml/verve/__ONLINE_playerSaved_EndStep.gml").into(),
            );

            println!("Adding extension...");
            assets.extensions.push(Extension {
                name: "GM Windows Dialogs".into(),
                folder_name: "".into(),
                files: vec![],
            });

            println!("Adding included sounds...");
            let snd_chatbox = include_bytes!("./res/__ONLINE_sndChatbox.wav");
            let snd_saved = include_bytes!("./res/__ONLINE_sndSaved.wav");
            assets.included_files.push(IncludedFile {
                file_name: "__ONLINE_sndChatbox.wav".into(),
                source_path: "__ONLINE_sndChatbox.wav".into(),
                data_exists: true,
                source_length: snd_chatbox.len(),
                stored_in_gmk: true,
                embedded_data: Some(Box::new(*snd_chatbox)),
                export_settings: gm8exe::asset::included_file::ExportSetting::NoExport,
                overwrite_file: true,
                free_memory: true,
                remove_at_end: true,
            });
            assets.included_files.push(IncludedFile {
                file_name: "__ONLINE_sndSaved.wav".into(),
                source_path: "__ONLINE_sndSaved.wav".into(),
                data_exists: true,
                source_length: snd_chatbox.len(),
                stored_in_gmk: true,
                embedded_data: Some(Box::new(*snd_saved)),
                export_settings: gm8exe::asset::included_file::ExportSetting::NoExport,
                overwrite_file: true,
                free_memory: true,
                remove_at_end: true,
            });

            println!("Adding font...");
            assets.fonts.push(Some(Box::new(Font {
                name: "__ONLINE_ftOnlinePlayerName".into(),
                sys_name: "Berlin Sans FB Demi".into(),
                size: 12,
                bold: false,
                italic: false,
                range_start: 32,
                range_end: 127,
                charset: 0,
                aa_level: 3,
                dmap: Box::new([0; 1536]),
                map_width: 0,
                map_height: 0,
                pixel_map: Box::new([]),
            })));

            println!("Patching objects...");
            for obj in assets.objects.iter_mut() {
                match obj {
                    Some(obj) => match obj.name.to_string().as_str() {
                        "World" => {
                            object_add_code(
                                obj,
                                EVENT_CREATE,
                                include_str!("./gml/verve/World_Create.gml")
                                    .replace("$GAME_ID", &game_id)
                                    .replace("$GAME_NAME", game_name)
                                    .replace("$SERVER_IP", server_ip)
                                    .as_str()
                                    .into(),
                            );
                            object_add_code(obj, EVENT_END_STEP, include_str!("./gml/verve/World_EndStep.gml").into());
                            object_add_code(obj, EVENT_GAME_END, include_str!("./gml/verve/World_GameEnd.gml").into());
                        },
                        _ => (),
                    },
                    _ => (),
                }
            }

            println!("Patching scripts...");
            for scr in assets.scripts.iter_mut() {
                match scr {
                    Some(scr) => match scr.name.to_string().as_str() {
                        "save_save" => {
                            scr.source = format!("{}\n{}", scr.source, include_str!("./gml/verve/save_save.gml"))
                                .as_str()
                                .into();
                        },
                        "save_load" => {
                            scr.source = format!("{}\n{}", scr.source, include_str!("./gml/verve/save_load.gml"))
                                .as_str()
                                .into();
                        },
                        _ => (),
                    },
                    _ => (),
                }
            }
        },
        Engine::Renex => {
            println!("Renex engine detected!");

            println!("Adding http dll scripts...");
            add_http_scripts(assets);

            println!("Adding online objects...");
            add_online_objects(
                assets,
                include_str!("./gml/renex/__ONLINE_onlinePlayer_Create.gml").into(),
                include_str!("./gml/renex/__ONLINE_onlinePlayer_EndStep.gml").into(),
                include_str!("./gml/renex/__ONLINE_onlinePlayer_Draw.gml").into(),
                include_str!("./gml/renex/__ONLINE_chatbox_Create.gml").into(),
                include_str!("./gml/renex/__ONLINE_chatbox_EndStep.gml").into(),
                include_str!("./gml/renex/__ONLINE_chatbox_Draw.gml").into(),
                include_str!("./gml/renex/__ONLINE_playerSaved_Draw.gml").into(),
                include_str!("./gml/renex/__ONLINE_playerSaved_EndStep.gml").into(),
            );

            println!("Adding extension...");
            assets.extensions.push(Extension {
                name: "GM Windows Dialogs".into(),
                folder_name: "".into(),
                files: vec![],
            });

            println!("Adding included sounds...");
            let snd_chatbox = include_bytes!("./res/__ONLINE_sndChatbox.wav");
            let snd_saved = include_bytes!("./res/__ONLINE_sndSaved.wav");
            assets.included_files.push(IncludedFile {
                file_name: "__ONLINE_sndChatbox.wav".into(),
                source_path: "__ONLINE_sndChatbox.wav".into(),
                data_exists: true,
                source_length: snd_chatbox.len(),
                stored_in_gmk: true,
                embedded_data: Some(Box::new(*snd_chatbox)),
                export_settings: gm8exe::asset::included_file::ExportSetting::NoExport,
                overwrite_file: true,
                free_memory: true,
                remove_at_end: true,
            });
            assets.included_files.push(IncludedFile {
                file_name: "__ONLINE_sndSaved.wav".into(),
                source_path: "__ONLINE_sndSaved.wav".into(),
                data_exists: true,
                source_length: snd_chatbox.len(),
                stored_in_gmk: true,
                embedded_data: Some(Box::new(*snd_saved)),
                export_settings: gm8exe::asset::included_file::ExportSetting::NoExport,
                overwrite_file: true,
                free_memory: true,
                remove_at_end: true,
            });

            println!("Adding font...");
            assets.fonts.push(Some(Box::new(Font {
                name: "__ONLINE_ftOnlinePlayerName".into(),
                sys_name: "Berlin Sans FB Demi".into(),
                size: 12,
                bold: false,
                italic: false,
                range_start: 32,
                range_end: 127,
                charset: 0,
                aa_level: 4,
                dmap: Box::new([0; 1536]),
                map_width: 0,
                map_height: 0,
                pixel_map: Box::new([]),
            })));

            println!("Patching objects...");
            for obj in assets.objects.iter_mut() {
                match obj {
                    Some(obj) => match obj.name.to_string().as_str() {
                        "World" => {
                            object_add_code(
                                obj,
                                EVENT_CREATE,
                                include_str!("./gml/renex/World_Create.gml")
                                    .replace("$GAME_ID", &game_id)
                                    .replace("$GAME_NAME", game_name)
                                    .replace("$SERVER_IP", server_ip)
                                    .as_str()
                                    .into(),
                            );
                            object_add_code(obj, EVENT_END_STEP, include_str!("./gml/renex/World_EndStep.gml").into());
                            object_add_code(obj, EVENT_GAME_END, include_str!("./gml/renex/World_GameEnd.gml").into());
                        },
                        _ => (),
                    },
                    _ => (),
                }
            }

            println!("Patching scripts...");
            for scr in assets.scripts.iter_mut() {
                match scr {
                    Some(scr) => match scr.name.to_string().as_str() {
                        "savedata_save" => {
                            scr.source = format!("{}\n{}", scr.source, include_str!("./gml/renex/savedata_save.gml"))
                                .as_str()
                                .into();
                        },
                        "savedata_load" => {
                            scr.source = format!("{}\n{}", scr.source, include_str!("./gml/renex/savedata_load.gml"))
                                .as_str()
                                .into();
                        },
                        _ => (),
                    },
                    _ => (),
                }
            }
        },
        Engine::Unknown => {
            panic!("Unsuppported engine! Please contact Cube.");
        },
    };
}

fn add_http_scripts(assets: &mut GameAssets) {
    assets.scripts.push(Some(Box::new(Script {
        name: "hbuffer_append_to_file".into(),
        source: include_str!("./gml/http/hbuffer_append_to_file.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hbuffer_at_end".into(),
        source: include_str!("./gml/http/hbuffer_at_end.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hbuffer_clear".into(),
        source: include_str!("./gml/http/hbuffer_clear.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hbuffer_clear_error".into(),
        source: include_str!("./gml/http/hbuffer_clear_error.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hbuffer_create".into(),
        source: include_str!("./gml/http/hbuffer_create.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hbuffer_destroy".into(),
        source: include_str!("./gml/http/hbuffer_destroy.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hbuffer_exists".into(),
        source: include_str!("./gml/http/hbuffer_exists.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hbuffer_get_error".into(),
        source: include_str!("./gml/http/hbuffer_get_error.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hbuffer_get_length".into(),
        source: include_str!("./gml/http/hbuffer_get_length.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hbuffer_get_pos".into(),
        source: include_str!("./gml/http/hbuffer_get_pos.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hbuffer_rc4_crypt".into(),
        source: include_str!("./gml/http/hbuffer_rc4_crypt.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hbuffer_rc4_crypt_buffer".into(),
        source: include_str!("./gml/http/hbuffer_rc4_crypt_buffer.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hbuffer_read_base64".into(),
        source: include_str!("./gml/http/hbuffer_read_base64.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hbuffer_read_data".into(),
        source: include_str!("./gml/http/hbuffer_read_data.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hbuffer_read_float32".into(),
        source: include_str!("./gml/http/hbuffer_read_float32.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hbuffer_read_float64".into(),
        source: include_str!("./gml/http/hbuffer_read_float64.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hbuffer_read_from_file".into(),
        source: include_str!("./gml/http/hbuffer_read_from_file.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hbuffer_read_from_file_part".into(),
        source: include_str!("./gml/http/hbuffer_read_from_file_part.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hbuffer_read_hex".into(),
        source: include_str!("./gml/http/hbuffer_read_hex.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hbuffer_read_int16".into(),
        source: include_str!("./gml/http/hbuffer_read_int16.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hbuffer_read_int32".into(),
        source: include_str!("./gml/http/hbuffer_read_int32.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hbuffer_read_int64".into(),
        source: include_str!("./gml/http/hbuffer_read_int64.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hbuffer_read_int8".into(),
        source: include_str!("./gml/http/hbuffer_read_int8.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hbuffer_read_intv".into(),
        source: include_str!("./gml/http/hbuffer_read_intv.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hbuffer_read_string".into(),
        source: include_str!("./gml/http/hbuffer_read_string.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hbuffer_read_uint16".into(),
        source: include_str!("./gml/http/hbuffer_read_uint16.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hbuffer_read_uint32".into(),
        source: include_str!("./gml/http/hbuffer_read_uint32.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hbuffer_read_uint64".into(),
        source: include_str!("./gml/http/hbuffer_read_uint64.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hbuffer_read_uint8".into(),
        source: include_str!("./gml/http/hbuffer_read_uint8.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hbuffer_read_uintv".into(),
        source: include_str!("./gml/http/hbuffer_read_uintv.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hbuffer_set_pos".into(),
        source: include_str!("./gml/http/hbuffer_set_pos.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hbuffer_to_string".into(),
        source: include_str!("./gml/http/hbuffer_to_string.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hbuffer_write_base64".into(),
        source: include_str!("./gml/http/hbuffer_write_base64.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hbuffer_write_buffer".into(),
        source: include_str!("./gml/http/hbuffer_write_buffer.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hbuffer_write_buffer_part".into(),
        source: include_str!("./gml/http/hbuffer_write_buffer_part.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hbuffer_write_data".into(),
        source: include_str!("./gml/http/hbuffer_write_data.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hbuffer_write_float32".into(),
        source: include_str!("./gml/http/hbuffer_write_float32.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hbuffer_write_float64".into(),
        source: include_str!("./gml/http/hbuffer_write_float64.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hbuffer_write_hex".into(),
        source: include_str!("./gml/http/hbuffer_write_hex.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hbuffer_write_int16".into(),
        source: include_str!("./gml/http/hbuffer_write_int16.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hbuffer_write_int32".into(),
        source: include_str!("./gml/http/hbuffer_write_int32.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hbuffer_write_int64".into(),
        source: include_str!("./gml/http/hbuffer_write_int64.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hbuffer_write_int8".into(),
        source: include_str!("./gml/http/hbuffer_write_int8.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hbuffer_write_intv".into(),
        source: include_str!("./gml/http/hbuffer_write_intv.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hbuffer_write_string".into(),
        source: include_str!("./gml/http/hbuffer_write_string.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hbuffer_write_to_file".into(),
        source: include_str!("./gml/http/hbuffer_write_to_file.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hbuffer_write_uint16".into(),
        source: include_str!("./gml/http/hbuffer_write_uint16.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hbuffer_write_uint32".into(),
        source: include_str!("./gml/http/hbuffer_write_uint32.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hbuffer_write_uint64".into(),
        source: include_str!("./gml/http/hbuffer_write_uint64.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hbuffer_write_uint8".into(),
        source: include_str!("./gml/http/hbuffer_write_uint8.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hbuffer_write_uintv".into(),
        source: include_str!("./gml/http/hbuffer_write_uintv.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hbuffer_zlib_compress".into(),
        source: include_str!("./gml/http/hbuffer_zlib_compress.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hbuffer_zlib_uncompress".into(),
        source: include_str!("./gml/http/hbuffer_zlib_uncompress.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hhttprequest_clear_post_parameters".into(),
        source: include_str!("./gml/http/hhttprequest_clear_post_parameters.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hhttprequest_clear_request_headers".into(),
        source: include_str!("./gml/http/hhttprequest_clear_request_headers.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hhttprequest_connect".into(),
        source: include_str!("./gml/http/hhttprequest_connect.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hhttprequest_create".into(),
        source: include_str!("./gml/http/hhttprequest_create.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hhttprequest_destroy".into(),
        source: include_str!("./gml/http/hhttprequest_destroy.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hhttprequest_exists".into(),
        source: include_str!("./gml/http/hhttprequest_exists.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hhttprequest_find_response_header".into(),
        source: include_str!("./gml/http/hhttprequest_find_response_header.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hhttprequest_get_message_body".into(),
        source: include_str!("./gml/http/hhttprequest_get_message_body.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hhttprequest_get_message_body_buffer".into(),
        source: include_str!("./gml/http/hhttprequest_get_message_body_buffer.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hhttprequest_get_message_body_length".into(),
        source: include_str!("./gml/http/hhttprequest_get_message_body_length.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hhttprequest_get_response_header_count".into(),
        source: include_str!("./gml/http/hhttprequest_get_response_header_count.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hhttprequest_get_response_header_name".into(),
        source: include_str!("./gml/http/hhttprequest_get_response_header_name.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hhttprequest_get_response_header_value".into(),
        source: include_str!("./gml/http/hhttprequest_get_response_header_value.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hhttprequest_get_state".into(),
        source: include_str!("./gml/http/hhttprequest_get_state.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hhttprequest_get_status_code".into(),
        source: include_str!("./gml/http/hhttprequest_get_status_code.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hhttprequest_remove_post_parameter".into(),
        source: include_str!("./gml/http/hhttprequest_remove_post_parameter.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hhttprequest_remove_request_header".into(),
        source: include_str!("./gml/http/hhttprequest_remove_request_header.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hhttprequest_reset".into(),
        source: include_str!("./gml/http/hhttprequest_reset.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hhttprequest_set_post_parameter".into(),
        source: include_str!("./gml/http/hhttprequest_set_post_parameter.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hhttprequest_set_post_parameter_file".into(),
        source: include_str!("./gml/http/hhttprequest_set_post_parameter_file.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hhttprequest_set_request_header".into(),
        source: include_str!("./gml/http/hhttprequest_set_request_header.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hhttprequest_update".into(),
        source: include_str!("./gml/http/hhttprequest_update.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hhttprequest_urldecode".into(),
        source: include_str!("./gml/http/hhttprequest_urldecode.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hhttprequest_urlencode".into(),
        source: include_str!("./gml/http/hhttprequest_urlencode.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hhttp_dll_init".into(),
        source: include_str!("./gml/http/hhttp_dll_init.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hlisteningsocket_accept".into(),
        source: include_str!("./gml/http/hlisteningsocket_accept.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hlisteningsocket_can_accept".into(),
        source: include_str!("./gml/http/hlisteningsocket_can_accept.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hlisteningsocket_create".into(),
        source: include_str!("./gml/http/hlisteningsocket_create.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hlisteningsocket_destroy".into(),
        source: include_str!("./gml/http/hlisteningsocket_destroy.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hlisteningsocket_exists".into(),
        source: include_str!("./gml/http/hlisteningsocket_exists.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hlisteningsocket_is_listening".into(),
        source: include_str!("./gml/http/hlisteningsocket_is_listening.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hlisteningsocket_start_listening".into(),
        source: include_str!("./gml/http/hlisteningsocket_start_listening.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hlisteningsocket_stop_listening".into(),
        source: include_str!("./gml/http/hlisteningsocket_stop_listening.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hmd5_begin".into(),
        source: include_str!("./gml/http/hmd5_begin.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hmd5_end".into(),
        source: include_str!("./gml/http/hmd5_end.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hmd5_read_buffer".into(),
        source: include_str!("./gml/http/hmd5_read_buffer.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hmd5_read_buffer_part".into(),
        source: include_str!("./gml/http/hmd5_read_buffer_part.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hmd5_read_file".into(),
        source: include_str!("./gml/http/hmd5_read_file.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hmd5_read_string".into(),
        source: include_str!("./gml/http/hmd5_read_string.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hmd5_result".into(),
        source: include_str!("./gml/http/hmd5_result.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hsha1_begin".into(),
        source: include_str!("./gml/http/hsha1_begin.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hsha1_end".into(),
        source: include_str!("./gml/http/hsha1_end.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hsha1_read_buffer".into(),
        source: include_str!("./gml/http/hsha1_read_buffer.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hsha1_read_buffer_part".into(),
        source: include_str!("./gml/http/hsha1_read_buffer_part.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hsha1_read_file".into(),
        source: include_str!("./gml/http/hsha1_read_file.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hsha1_read_string".into(),
        source: include_str!("./gml/http/hsha1_read_string.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hsha1_result".into(),
        source: include_str!("./gml/http/hsha1_result.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hsocket_connect".into(),
        source: include_str!("./gml/http/hsocket_connect.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hsocket_create".into(),
        source: include_str!("./gml/http/hsocket_create.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hsocket_destroy".into(),
        source: include_str!("./gml/http/hsocket_destroy.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hsocket_exists".into(),
        source: include_str!("./gml/http/hsocket_exists.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hsocket_get_peer_address".into(),
        source: include_str!("./gml/http/hsocket_get_peer_address.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hsocket_get_read_data_length".into(),
        source: include_str!("./gml/http/hsocket_get_read_data_length.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hsocket_get_state".into(),
        source: include_str!("./gml/http/hsocket_get_state.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hsocket_get_write_data_length".into(),
        source: include_str!("./gml/http/hsocket_get_write_data_length.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hsocket_read_data".into(),
        source: include_str!("./gml/http/hsocket_read_data.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hsocket_read_message".into(),
        source: include_str!("./gml/http/hsocket_read_message.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hsocket_read_message_delimiter".into(),
        source: include_str!("./gml/http/hsocket_read_message_delimiter.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hsocket_reset".into(),
        source: include_str!("./gml/http/hsocket_reset.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hsocket_shut_down".into(),
        source: include_str!("./gml/http/hsocket_shut_down.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hsocket_update_read".into(),
        source: include_str!("./gml/http/hsocket_update_read.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hsocket_update_write".into(),
        source: include_str!("./gml/http/hsocket_update_write.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hsocket_write_data".into(),
        source: include_str!("./gml/http/hsocket_write_data.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hsocket_write_message".into(),
        source: include_str!("./gml/http/hsocket_write_message.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hsocket_write_message_delimiter".into(),
        source: include_str!("./gml/http/hsocket_write_message_delimiter.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hudpsocket_create".into(),
        source: include_str!("./gml/http/hudpsocket_create.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hudpsocket_destroy".into(),
        source: include_str!("./gml/http/hudpsocket_destroy.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hudpsocket_exists".into(),
        source: include_str!("./gml/http/hudpsocket_exists.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hudpsocket_get_last_address".into(),
        source: include_str!("./gml/http/hudpsocket_get_last_address.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hudpsocket_get_last_port".into(),
        source: include_str!("./gml/http/hudpsocket_get_last_port.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hudpsocket_get_max_message_size".into(),
        source: include_str!("./gml/http/hudpsocket_get_max_message_size.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hudpsocket_get_state".into(),
        source: include_str!("./gml/http/hudpsocket_get_state.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hudpsocket_receive".into(),
        source: include_str!("./gml/http/hudpsocket_receive.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hudpsocket_reset".into(),
        source: include_str!("./gml/http/hudpsocket_reset.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hudpsocket_send".into(),
        source: include_str!("./gml/http/hudpsocket_send.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hudpsocket_set_destination".into(),
        source: include_str!("./gml/http/hudpsocket_set_destination.gml").into(),
    })));

    assets.scripts.push(Some(Box::new(Script {
        name: "hudpsocket_start".into(),
        source: include_str!("./gml/http/hudpsocket_start.gml").into(),
    })));
}

const EVENT_CREATE: (usize, u32) = (0, 0);
const EVENT_STEP: (usize, u32) = (3, 0);
const EVENT_END_STEP: (usize, u32) = (3, 2);
const EVENT_DRAW: (usize, u32) = (8, 0);
const EVENT_GAME_END: (usize, u32) = (7, 3);

fn object_add_code(obj: &mut Object, (event_index, subevent_index): (usize, u32), code: PascalString) {
    let code_action = CodeAction {
        id: 603,
        applies_to: -1,
        is_condition: false,
        invert_condition: false,
        is_relative: false,
        lib_id: 1,
        action_kind: 7,
        execution_type: 2,
        can_be_relative: 0,
        applies_to_something: true,
        fn_name: PascalString::default(),
        fn_code: PascalString::default(),
        param_count: 1,
        param_types: [1, 0, 0, 0, 0, 0, 0, 0],
        param_strings: [
            code,
            PascalString::default(),
            PascalString::default(),
            PascalString::default(),
            PascalString::default(),
            PascalString::default(),
            PascalString::default(),
            PascalString::default(),
        ],
    };

    let subevents = obj.events.get_mut(event_index).unwrap();

    // Subevent exists
    for (sub, actions) in subevents.iter_mut() {
        if *sub == subevent_index {
            actions.push(code_action);
            return;
        }
    }

    // Subevent not exists
    subevents.push((subevent_index, vec![code_action]));
}

fn add_online_objects(
    assets: &mut GameAssets,
    online_player_create: PascalString,
    online_player_endstep: PascalString,
    online_player_draw: PascalString,
    chatbox_create: PascalString,
    chatbox_endstep: PascalString,
    chatbox_draw: PascalString,
    player_saved_draw: PascalString,
    player_saved_endstep: PascalString,
) {
    let mut online_player = Object {
        name: "__ONLINE_onlinePlayer".into(),
        visible: false,
        depth: -10,
        sprite_index: -1,
        mask_index: -1,
        parent_index: -1,
        solid: false,
        persistent: true,
        events: vec![vec![], vec![], vec![], vec![], vec![], vec![], vec![], vec![], vec![], vec![], vec![]],
    };
    object_add_code(&mut online_player, EVENT_CREATE, online_player_create);
    object_add_code(&mut online_player, EVENT_END_STEP, online_player_endstep);
    object_add_code(&mut online_player, EVENT_DRAW, online_player_draw);
    assets.objects.push(Some(Box::new(online_player)));

    let mut online_chatbox = Object {
        name: "__ONLINE_chatbox".into(),
        visible: true,
        depth: -11,
        sprite_index: -1,
        mask_index: -1,
        parent_index: -1,
        solid: false,
        persistent: true,
        events: vec![vec![], vec![], vec![], vec![], vec![], vec![], vec![], vec![], vec![], vec![], vec![]],
    };
    object_add_code(&mut online_chatbox, EVENT_CREATE, chatbox_create);
    object_add_code(&mut online_chatbox, EVENT_END_STEP, chatbox_endstep);
    object_add_code(&mut online_chatbox, EVENT_DRAW, chatbox_draw);
    assets.objects.push(Some(Box::new(online_chatbox)));

    let mut online_player_saved = Object {
        name: "__ONLINE_playerSaved".into(),
        visible: true,
        depth: -10,
        sprite_index: -1,
        mask_index: -1,
        parent_index: -1,
        solid: false,
        persistent: false,
        events: vec![vec![], vec![], vec![], vec![], vec![], vec![], vec![], vec![], vec![], vec![], vec![]],
    };
    object_add_code(&mut online_player_saved, EVENT_END_STEP, player_saved_endstep);
    object_add_code(&mut online_player_saved, EVENT_DRAW, player_saved_draw);
    assets.objects.push(Some(Box::new(online_player_saved)));
}
