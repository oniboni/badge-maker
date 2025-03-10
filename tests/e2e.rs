use badge_maker::BadgeBuilder;
mod constants;
use constants::*;
use seahash::hash;
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};

const USE_CACHE: bool = true;

fn get_node_output(badge: &NewBadge) -> String {
    let file = "badge-cli.js";
    let mut node_badge_maker = std::process::Command::new("node");
    node_badge_maker.arg(file);

    let style = match badge.style.as_ref() {
        "plastic" => "@plastic",
        "flatsquare" => "@flat-square",
        "flat" => "@flat",
        _ => panic!("no style!"),
    };

    node_badge_maker.args(&[
        &badge.label,
        &badge.message,
        &badge.color,
        &badge.label_color,
        style,
    ]);
    node_badge_maker.spawn().unwrap();

    String::from_utf8(node_badge_maker.output().unwrap().stdout).unwrap()
}

fn clean(svg: &str, replace_id: &str) -> String {
    let new_s = format!(r#"id="bms-{}""#, replace_id);
    let new_r = format!(r#"id="bmr-{}""#, replace_id);

    let new_s_use = format!(r#"fill="url(#bms-{})""#, replace_id);
    let new_r_use = format!(r#"clip-path="url(#bmr-{})""#, replace_id);

    let ids = r#"id="s""#;

    let idr = r#"id="r""#;

    let s_use = r#"fill="url(#s)""#;
    let r_use = r#"clip-path="url(#r)""#;

    svg.replace(ids, &new_s)
        .replace(idr, &new_r)
        .replace(r_use, &new_r_use)
        .replace(s_use, &new_s_use)
        .replace("\n", "")
}

fn load(badge_id: u64) -> (HashMap<String, String>, bool) {
    if !USE_CACHE {
        return (HashMap::new(), false);
    }

    let file = File::open(format!("./cache/{}", badge_id));

    match file {
        Ok(mut file) => {
            let mut bytes = vec![];
            file.read_to_end(&mut bytes).unwrap();
            (bincode::deserialize(&mut bytes).unwrap(), true)
        }
        Err(_) => {
            std::fs::remove_dir_all("./cache").unwrap();
            std::fs::create_dir("./cache").unwrap();
            (HashMap::new(), false)
        }
    }
}

#[test]
fn e2e() {
    let path = std::path::Path::new(std::env::current_dir().unwrap().as_path())
        .join("tests/node_badge_maker/");

    if !std::path::Path::exists("tests/node_badge_maker".as_ref()) {
        std::env::set_current_dir("tests").unwrap();
        #[cfg(not(windows))]
        match std::process::Command::new("unzip")
            .arg("node_badge_maker.zip")
            .spawn()
        {
            Ok(p) => p.wait_with_output().unwrap(),
            Err(_) => {
                eprintln!("unable to unzip. Do you have unzip?");
                return;
            }
        };
        #[cfg(windows)]
        match std::process::Command::new("tar")
            .arg("-xf")
            .arg("node_badge_maker.zip")
            .spawn()
        {
            Ok(p) => p.wait_with_output().unwrap(),
            Err(_) => {
                eprintln!("unable to unzip. Do you have tar in windows?");
                return;
            }
        };
        std::env::set_current_dir("node_badge_maker").unwrap();
    } else {
        std::env::set_current_dir(path).unwrap();
    }

    let badges = get_badges();
    // let badges = badges[0..20].to_vec();
    let badge_bytes = bincode::serialize(&badges).unwrap();
    let file_name = hash(&badge_bytes);

    let (mut results, did_load) = load(file_name);

    println!(
        "\t\tchecking {} badges for deviations from Node badge-maker...",
        badges.len()
    );

    for badge in &badges {
        let rust = BadgeBuilder::new()
            .label(&badge.label)
            .message(&badge.message)
            .label_color_parse(&badge.label_color)
            .color_parse(&badge.color)
            .style_parse(&badge.style)
            .build()
            .unwrap();

        let test_id = rust.id().to_string();

        let node = results
            .entry(test_id.clone())
            .or_insert_with(|| clean(&get_node_output(&badge), &test_id));

        let rust = rust.svg();

        assert_eq!(node.to_string(), rust);
    }

    println!("\t\tall badges matched");

    if !did_load && USE_CACHE {
        let mut file = File::create(format!("./cache/{}", file_name)).unwrap();
        let bytes = bincode::serialize(&results).unwrap();
        file.write(&bytes).unwrap();
    }
}
