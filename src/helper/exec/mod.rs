/// Command Execution
// Local imports
use crate::{
    helper::{
        check_arg_len,
        colored::Colorize,
        read_file,
        resource::{calculate_hash, continue_prompt, input_fmt, read_file_gpath},
        usage_and_quit, verbose_check, verbose_info_print, Tool, ToolInstallMethod, ZzzConfig,
    },
    list, STARTCMD,
};

use crate::helper::errors::*;

// std imports
use std::{
    env,
    error::Error,
    fs,
    fs::File,
    io::{BufReader, Write},
    process::Command,
};

use super::resource::quit;

pub fn list_exec(
    v_file: File,
    filepath: String,
    way: usize,
    global_opts: &[bool],
) -> Result<(), Box<dyn Error>> {
    let reader: BufReader<File> = BufReader::new(v_file);
    // Parse the YAML
    let config: Result<ZzzConfig, serde_yaml::Error> = serde_yaml::from_reader(reader);
    match config {
        Err(_) => {
            INVALIDFILEERR.show_error(&filepath, global_opts);
            Err("Invalid Config".into())
        }

        Ok(config) => match way {
            1 => {
                if config.DEPENDANCIES.TOOLS.is_empty() {
                    warnprint!("Dream: '{}' has no dependancies!", filepath);
                    Ok(())
                } else {
                    infoprint!("'{}' requires the following dependancies:", filepath);
                    let mut num = 1;
                    for tool in config.DEPENDANCIES.TOOLS {
                        println!("\t{0}: {1} \t (from {2})", num, tool.NAME, tool.LINK);
                        num += 1;
                    }
                    Ok(())
                }
            }

            _ => {
                infoprint!("Dependancies for {}:", filepath);
                let mut num = 1;
                for tool in config.DEPENDANCIES.TOOLS {
                    println!("\t{0}: {1}", num, tool.NAME);
                    num += 1;
                }
                Ok(())
            }
        },
    }
}

pub fn add_exec(
    filepath: &String,
    depname: &String,
    method: ToolInstallMethod,
    global_opts: &[bool],
) -> Result<(), Box<dyn Error>> {
    let link = questionprint!("Enter link for '{}':", depname);
    match read_file_gpath(filepath) {
        Ok(v_file) => {
            let config: Result<ZzzConfig, serde_yaml::Error> = serde_yaml::from_reader(&v_file.0);
            let mut conf_f = config.unwrap();
            let n_tool: Tool = Tool {
                NAME: depname.to_string(),
                LINK: link,
                METHOD: method,
            };
            let mut tool_to_add: Vec<Tool> = vec![n_tool];
            conf_f.DEPENDANCIES.TOOLS.append(&mut tool_to_add);
            conf_f.PROJECT.IS_LOADED = false;
            let f = std::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .open(v_file.1)
                .expect("Couldn't open file");
            serde_yaml::to_writer(f, &conf_f).unwrap();
        }
        Err(file) => {
            println!("{}.", file.0);
            MISSINGFILEERROR.show_error(&file.1, global_opts);
        }
    };

    successprint!("'{0}' added to {1}", depname, &filepath);

    Ok(())
}

pub fn remove_exec(
    filepath: &String,
    depname: &String,
    global_opts: &[bool],
) -> Result<(), Box<dyn Error>> {
    match read_file_gpath(filepath) {
        Ok(v_file) => {
            let config: Result<ZzzConfig, serde_yaml::Error> = serde_yaml::from_reader(&v_file.0);
            let mut conf_f = config.unwrap();
            let toollist = &mut conf_f.DEPENDANCIES.TOOLS;
            let index = toollist.iter().position(|x| x.NAME == *depname).unwrap();
            warnprint!(
                "This will remove {} from {}",
                toollist[index].NAME,
                filepath
            );
            continue_prompt(global_opts);
            toollist.remove(index);
            conf_f.PROJECT.IS_LOADED = false;
            let f = std::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(v_file.1)
                .expect("Couldn't open file");
            serde_yaml::to_writer(f, &conf_f).unwrap();
        }
        Err(file) => {
            MISSINGFILEERROR.show_error(&file.1, global_opts);
        }
    };

    successprint!("'{0}' removed from {1}", depname, &filepath);

    Ok(())
}

pub fn load_exec(
    v_file: File,
    filepath: String,
    mut env_cmds: Vec<String>,
    mut home_dir: Result<String, env::VarError>,
    global_opts: &[bool],
    argsv: Vec<String>,
) -> Result<(Vec<String>, u64), Box<dyn Error>> {
    let reader: BufReader<File> = BufReader::new(v_file);
    // Parse the YAML into DepConfig struct
    let config: Result<ZzzConfig, serde_yaml::Error> = serde_yaml::from_reader(reader);
    match config {
        Err(_) => {
            INVALIDFILEERR.show_error(&filepath, global_opts);
            Err("Invalid Config".into())
        }
        Ok(mut config) => {
            let hashname = calculate_hash(&config.PROJECT.NAME);
            //println!("{}", hash_string(&config.project.name));
            if !config.PROJECT.IS_LOADED || global_opts[2] {
                let _ = list(argsv.clone(), 1, global_opts);
                if global_opts[2] {
                    infoprint!("This action will download the above, and run any tasks included.");
                }
                continue_prompt(global_opts);
                infoprint!("Getting dependancies from file: '{}'", filepath);
                for tool in &config.DEPENDANCIES.TOOLS {
                    let _ = tool_install(tool, hashname, &mut env_cmds, &mut home_dir, global_opts);
                }
                config.PROJECT.IS_LOADED = true;
                let f = std::fs::OpenOptions::new()
                    .write(true)
                    .create(true)
                    .open(filepath)
                    .expect("Couldn't open file");
                serde_yaml::to_writer(f, &config).unwrap();
            }
            let result = (env_cmds, hashname);
            Ok(result)
        }
    }
}

pub fn load_deps(
    argsv: Vec<String>,
    env_cmds: &[String],
    home_dir: Result<String, env::VarError>,
    global_opts: &[bool],
) -> Result<(Vec<String>, u64), Box<dyn Error>> {
    if check_arg_len(argsv.clone(), 2) {
        usage_and_quit(STARTCMD.name, "Missing Filename!");
        return Err("Bad File".into());
    } else {
        let _: Result<(Vec<String>, u64), ()> = match read_file(&argsv, 2, STARTCMD) {
            Ok(v_file) => {
                let result = load_exec(
                    v_file.0,
                    v_file.1,
                    env_cmds.to_vec(),
                    home_dir,
                    global_opts,
                    argsv,
                );

                return result;
                //Ok(result)
            }
            Err(file) => {
                MISSINGFILEERROR.show_error(&file.1, global_opts);
                Err(())
            }
        };
    }
    Err("Bad File".into())
}

fn tool_install(
    tool: &Tool,
    hashname: u64,
    env_cmds: &mut Vec<String>,
    home_dir: &mut Result<String, env::VarError>,
    global_opts: &[bool],
) -> Result<(), Box<dyn Error>> {
    env_cmds.push(tool.NAME.clone());
    verbose_info_print(
        format!("Installing {0} from {1}", tool.NAME, tool.LINK),
        global_opts,
    );
    let link = &tool.LINK;
    let method = &tool.METHOD;
    let link_str = link.to_string();
    match method {
        ToolInstallMethod::LINKZIP => {
            if cfg!(windows) {
                let dir_loc = format!(
                    "{0}\\.snooze\\bins\\{1}\\",
                    home_dir.as_mut().unwrap(),
                    hashname
                );
                match fs::create_dir_all(&dir_loc) {
                    Ok(..) => {
                        let namef = format!("{0}{1}", dir_loc, tool.NAME);
                        let args: Vec<&str> =
                            vec!["/C", "curl", &link_str, "--output", &namef, "--silent"];
                        //println!("{:?}", args);

                        let status = Command::new("cmd").args(args).status()?;
                        if status.success() {
                            let args2: Vec<&str> = vec!["/C", "chmod", "a+x", &namef];
                            let status2 = Command::new("cmd").args(args2).status()?;
                            if status2.success() {
                                verbose_info_print(
                                    format!("'{}' installed", tool.NAME),
                                    global_opts,
                                );
                                Ok(())
                            } else {
                                errprint!("Error grabbing: '{}'", tool.NAME);
                                continue_prompt(global_opts);
                                Err("Error grabbing".into())
                            }
                            //infoprint!("Command '{}' executed successfully", command);
                        } else {
                            errprint!("Error grabbing: '{}'", tool.NAME);
                            continue_prompt(global_opts);
                            Err("Error grabbing".into())
                        }
                    }
                    Err(..) => {
                        errprint!("Error creating dir");
                        Err("Error creating dir".into())
                    }
                }
            } else {
                let dir_loc = format!(
                    "{0}/.snooze/bins/{1}/",
                    home_dir.as_mut().unwrap(),
                    hashname
                );
                match fs::create_dir_all(&dir_loc) {
                    Ok(..) => {
                        let link_str_f = link_str.to_string();
                        let namef = format!("{0}{1}", dir_loc, tool.NAME);
                        let args: Vec<&str> =
                            vec!["-c", "/usr/bin/curl", &link_str_f, "--output", &namef];
                        let status = Command::new("bash").args(args).status()?;

                        if status.success() {
                            let args2: Vec<&str> = vec!["-c", "chmod", "a+x", &namef];
                            let status2 = Command::new("bash").args(args2).status()?;
                            if status2.success() {
                                verbose_info_print(
                                    format!("'{}' installed", tool.NAME),
                                    global_opts,
                                );
                                Ok(())
                            } else {
                                errprint!("Error grabbing: '{}'", tool.NAME);
                                Err("Error grabbing".into())
                            }
                        } else {
                            errprint!("Error grabbing: '{}'", tool.NAME);
                            Err("Error grabbing".into())
                        }
                    }
                    Err(..) => {
                        errprint!("Error creating dir");
                        Err("Error creating dir".into())
                    }
                }
            }
        }
        &ToolInstallMethod::GIT => {
            if cfg!(windows) {
                let dir_loc = format!(
                    "{0}\\.snooze\\bins\\{1}\\",
                    home_dir.as_mut().unwrap(),
                    hashname
                );
                let dir_temp = format!(
                    "{0}\\.snooze\\ztemp\\{1}\\",
                    home_dir.as_mut().unwrap(),
                    hashname
                );
                match fs::create_dir_all(&dir_loc) {
                    Ok(..) => {
                        let curr_dir = env::current_dir().unwrap();
                        let _namef = format!("{0}{1}", dir_loc, tool.NAME);
                        let args: Vec<&str> =
                            vec!["/C", "git", "clone", &link_str, &dir_temp, "-q"];
                        verbose_info_print("Cloning repo...".to_string(), global_opts);
                        let status = Command::new("cmd").args(args).status()?;
                        if status.success() {
                            match env::set_current_dir(&dir_temp) {
                                Ok(()) => {
                                    println!("Made it to {}", &dir_temp);
                                    let argsv: Vec<String> =
                                        vec!["".to_string(), "".to_string(), "dream".to_string()];
                                    let _: Result<(), Box<dyn Error>> =
                                        match read_file(&argsv, 2, STARTCMD) {
                                            Ok(v_file) => {
                                                let reader: BufReader<File> =
                                                    BufReader::new(v_file.0);

                                                let nconfig: Result<ZzzConfig, serde_yaml::Error> =
                                                    serde_yaml::from_reader(reader);

                                                let args: Vec<&str> =
                                                    vec!["/C", "zzz", "run", "dream"];
                                                verbose_info_print(
                                                    "Building...".to_string(),
                                                    global_opts,
                                                );
                                                let status =
                                                    Command::new("cmd").args(args).status()?;
                                                if status.success() {
                                                    let package = &nconfig.unwrap().PROJECT.PACKAGE;
                                                    let install_loc = format!(
                                                        "{}/.snooze/bins/{}/",
                                                        home_dir.as_ref().unwrap(),
                                                        hashname
                                                    );
                                                    let args: Vec<&str> =
                                                        vec!["/C", "cp", package, &install_loc];
                                                    verbose_info_print(
                                                        "Building...".to_string(),
                                                        global_opts,
                                                    );
                                                    let ctemp = env::current_dir().unwrap();
                                                    let status =
                                                        Command::new("cmd").args(args).status()?;
                                                    if status.success() {
                                                        env::set_current_dir(curr_dir)?;
                                                        fs::remove_dir_all(ctemp)?;
                                                        Ok(())
                                                    } else {
                                                        quit(4);
                                                        Err("asdf".into())
                                                    }
                                                } else {
                                                    quit(4);
                                                    Err("asdf".into())
                                                }
                                            }
                                            Err(file) => {
                                                MISSINGFILEERROR.show_error(&file.1, global_opts);
                                                Err("Missing File".into())
                                            }
                                        };
                                    Ok(())
                                }
                                Err(..) => {
                                    quit(4);
                                    Err("asdf".into())
                                }
                            }
                        } else {
                            errprint!("Error grabbing: '{}'", tool.NAME);
                            continue_prompt(global_opts);
                            return Err("Error grabbing".into());
                        }
                    }
                    Err(..) => {
                        errprint!("Error creating dir");
                        return Err("Error creating dir".into());
                    }
                }
            } else {
                let dir_loc = format!(
                    "{0}/.snooze/bins/{1}/",
                    home_dir.as_mut().unwrap(),
                    hashname
                );
                match fs::create_dir_all(&dir_loc) {
                    Ok(..) => {
                        let link_str_f = link_str.to_string();
                        let namef = format!("{0}{1}", dir_loc, tool.NAME);
                        let args: Vec<&str> =
                            vec!["-c", "/usr/bin/curl", &link_str_f, "--output", &namef];
                        let status = Command::new("bash").args(args).status()?;

                        if status.success() {
                            let args2: Vec<&str> = vec!["-c", "chmod", "a+x", &namef];
                            let status2 = Command::new("bash").args(args2).status()?;
                            if status2.success() {
                                verbose_info_print(
                                    format!("'{}' installed", tool.NAME),
                                    global_opts,
                                );
                                return Ok(());
                            } else {
                                errprint!("Error grabbing: '{}'", tool.NAME);
                                return Err("Error grabbing".into());
                            }
                        } else {
                            errprint!("Error grabbing: '{}'", tool.NAME);
                            return Err("Error grabbing".into());
                        }
                    }
                    Err(..) => {
                        errprint!("Error creating dir");
                        return Err("Error creating dir".into());
                    }
                }
            }
        }
    }
}

pub fn run_exec(
    v_file: File,
    filepath: String,
    global_opts: Vec<bool>,
) -> Result<(), Box<dyn Error>> {
    let reader: BufReader<File> = BufReader::new(v_file);
    // Parse the YAML into PluConfig struct
    let config: Result<ZzzConfig, serde_yaml::Error> = serde_yaml::from_reader(reader);
    match config {
        Err(_) => {
            MISSINGFILEERROR.show_error(&filepath, &global_opts);
            Err("Invalid Config".into())
        }

        Ok(config) => {
            let mut okcount: i32 = 0;
            let mut cmdcount: i32 = 0;
            // Execute commands in the 'run' section
            infoprint!("Running '{}': \n", filepath);
            for command in config.ON.RUN {
                cmdcount += 1;
                let mut parts = command.split_whitespace();
                let program = parts.next().ok_or("Missing command")?;
                let args: Vec<&str> = parts.collect();
                let status = Command::new(program).args(args).status()?;
                if status.success() {
                    if verbose_check(&global_opts) {
                        verbose_info_print("ok".to_string(), &global_opts);
                    }
                    okcount += 1;
                } else {
                    errprint!("Error executing command: '{}'", command);
                    continue_prompt(&global_opts);
                }
            }
            if cmdcount == okcount {
                println!();
                successprint!("All tasks completed successfully");
            }
            Ok(())
        }
    }
}

pub fn createfile(
    ufile_name: String,
    zfile_name_f: &String,
) -> Result<std::string::String, std::string::String> {
    infoprint!("Creating file: {}", ufile_name);
    let mut ufile = File::create(&ufile_name).expect("[!] Error encountered while creating file!");
    let content = format!(
        "PROJECT: {{
  NAME: \"{}\",
  DESCRIPTION: \"\",
  VERSION: \"0.0.0\",
  IS_LOADED: false,
}}

ON:
  RUN:
    - echo hello world

DEPENDANCIES:
  TOOLS:",
        &zfile_name_f
    );
    ufile
        .write_all(content.as_bytes())
        .expect("[!] Error while writing to file");

    Ok("File Created!".to_string())
}

pub fn extension_exec(
    argsv: &Vec<String>,
    home_dir: Result<String, env::VarError>,
    global_opts: &[bool],
) -> Result<(), String> {
    let mut to_exec: String;
    let argslen = &argsv.len();
    let ext_args: Vec<String>;
    if argslen < &1 {
        quit(4);
        Err("Not enough args".into())
    } else {
        match argslen {
            &1 => {
                ext_args = vec![];
            }
            _ => {
                ext_args = argsv.clone().drain(2..*argslen).collect();
            }
        }
        if cfg!(windows) {
            to_exec = format!("{}/.snooze/ext/{}.exe", home_dir.unwrap(), &argsv[1]).to_owned();
            let f_fexec = str::replace(&to_exec, "\\", "/").to_owned();
            to_exec = f_fexec.to_owned();
        } else {
            to_exec = format!("{}/.snooze/ext/{}", home_dir.unwrap(), &argsv[1].to_owned());
        }
        verbose_info_print(format!("Executing {}", to_exec).to_string(), global_opts);
        let status = Command::new(to_exec.clone()).args(ext_args).status();
        match status {
            Ok(_val) => Ok(()),
            Err(..) => {
                INVALIDEXTERROR.show_error(&to_exec, global_opts);
                Err("Bad command exec".into())
            }
        }
    }
}
