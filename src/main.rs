// WARNING: this contains some of the worst code ive written
//
// its hacky and still works but please
// dont take it as an example of me trying to make something good

use std::{collections::HashMap, fs::File, io::Write, path::PathBuf};

use anyhow::Result as AResult;
use arcsys::{
    ggacpr::replay::{AcprReplay, Character, MatchResult},
    BinRead,
};
use clap::Parser;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use walkdir::WalkDir;

/// ACPR Stats CLI - Written by Pangaea
#[derive(Parser)]
struct CliArgs {
    /// The path to your +R replays folder
    replay_folder: PathBuf,
    /// The Steam 64-bit ID that should be used for analysis.
    /// By default the program uses the most commonly occuring Steam ID
    steam_id: Option<u64>,
}

fn main() {
    match run() {
        Ok(_) => {}
        Err(e) => println!("Error: {}", e),
    }
}

fn run() -> AResult<()> {
    let args = CliArgs::parse();

    let walkdir_iter = WalkDir::new(args.replay_folder)
        .into_iter()
        .filter_map(|x| {
            x.ok().and_then(|x| {
                let path = x.into_path();

                if !path.is_file() {
                    return None;
                }

                Some(path)
            })
        });

    let paths = walkdir_iter.collect::<Vec<PathBuf>>();

    let replays = paths
        .into_par_iter()
        .filter_map(|p| {
            File::open(p)
                .ok()
                .and_then(|mut f| AcprReplay::read(&mut f).ok())
        })
        .collect::<Vec<AcprReplay>>();

    if replays.is_empty() {
        anyhow::bail!("No valid replay files found!")
    }

    let player_id = if let Some(id) = args.steam_id {
        id
    } else {
        let mut steam_id_counts = HashMap::new();

        for replay in &replays {
            steam_id_counts
                .entry(replay.p1_steam_id)
                .and_modify(|e: &mut usize| *e += 1)
                .or_insert(1);
            steam_id_counts
                .entry(replay.p2_steam_id)
                .and_modify(|e: &mut usize| *e += 1)
                .or_insert(1);
        }

        // get most frequently occurring player ID
        // this most likely will be the same as the person using this tool
        let player_id = steam_id_counts
            .into_iter()
            .max_by(|(_, count_a), (_, count_b)| count_a.cmp(count_b))
            .unwrap()
            .0;

        player_id
    };

    let results = replays
        .into_par_iter()
        // filter matches not including our main player
        .filter(|r| r.p1_steam_id == player_id || r.p2_steam_id == player_id)
        // filter out desynced/disconnected/unfinished matches
        .filter(|r| (!r.match_desynced || !r.match_disconnected || r.match_unfinished))
        // filter out draw matches
        .filter(|r| {
            if let MatchResult::Draw = r.match_result {
                false
            } else {
                true
            }
        })
        .map(|r| {
            let is_p1 = r.p1_steam_id == player_id;

            let character_played = if is_p1 {
                r.p1_character
            } else {
                r.p2_character
            };

            let opponent_character = if is_p1 {
                r.p2_character
            } else {
                r.p1_character
            };

            let match_won = if is_p1 {
                if let MatchResult::P1Winner = r.match_result {
                    true
                } else {
                    false
                }
            } else {
                if let MatchResult::P1Winner = r.match_result {
                    false
                } else {
                    true
                }
            };

            let score = if is_p1 { r.p1_score } else { r.p2_score };
            (character_played, opponent_character, match_won, score)
        })
        .collect::<Vec<(Character, Character, bool, u8)>>();

    let mut map = HashMap::new();

    for (character, opponent, match_won, _score) in results {
        let character_entry = map.entry(character).or_insert(HashMap::new());
        let results = character_entry
            .entry(opponent)
            .or_insert(WinRatio::default());

        results.add_result(match_won);
    }

    // fuck it lol why not
    let character_list: Vec<Character> = (1..=25)
        .into_iter()
        .map(|n| unsafe { std::mem::transmute(n as u8) })
        .collect();

    let mut csv = String::from("Player Character, ");
    for character in character_list.iter() {
        csv.push_str(format!("VS {}, ", character_to_str(*character)).as_str());
    }

    csv.push('\n');

    for character in character_list.iter() {
        if let Some(matchup_map) = map.get(&character) {
            csv.push_str(format!("{}, ", character_to_str(*character)).as_str());

            character_list.iter().for_each(|c| {
                let a = matchup_map.get(c).map_or("N/A".to_string(), |w| {
                    w.get_ratio()
                        .map_or("N/A".to_string(), |r| format!("{:.2}", r))
                });
                csv.push_str(format!("{}, ", a).as_str());
            })
        } else {
            continue;
        }

        csv.push('\n');
    }

    //println!("{}", csv);

    std::fs::File::create("./ACPR_REPLAY_MATCHUPS.csv")
        .and_then(|mut f| f.write_all(csv.as_bytes()))?;

    Ok(())
}

#[derive(Default)]
struct WinRatio {
    wins: u32,
    losses: u32,
}

impl WinRatio {
    pub fn add_result(&mut self, match_won: bool) {
        if match_won {
            self.wins += 1
        } else {
            self.losses += 1
        }
    }

    pub fn get_ratio(&self) -> Option<f32> {
        if self.wins == 0 && self.losses == 0 {
            return None;
        }

        Some(self.wins as f32 / (self.wins as f32 + self.losses as f32))
    }
}

fn character_to_str(char: Character) -> String {
    use Character::*;
    let c = match char {
        Sol => "Sol",
        Ky => "Ky",
        May => "May",
        Millia => "Millia",
        Axl => "Axl",
        Potemkin => "Potemkin",
        Chipp => "Chipp",
        Eddie => "Eddie",
        Baiken => "Baiken",
        Faust => "Faust",
        Testament => "Testament",
        Jam => "Jam",
        Anji => "Anji",
        Johnny => "Johnny",
        Venom => "Venom",
        Dizzy => "Dizzy",
        Slayer => "Slayer",
        Ino => "I-No",
        Zappa => "Zappa",
        Bridget => "Bridget",
        RoboKy => "Robo-Ky",
        Aba => "Aba",
        OrderSol => "Order-Sol",
        Kliff => "Kliff",
        Justice => "Justice",
    };

    c.into()
}
