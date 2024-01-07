use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::time::Duration;
use robotevents::*;
use robotevents::query::{DivisionMatchesQuery, EventsQuery, PaginatedQuery};
use robotevents::schema::{AllianceTeam, EventType, Match, PaginatedResponse, IdInfo};
use chrono::{DateTime};
use serde::{Serialize, Deserialize};
use crate::CompletedMatchError::{MatchNotScored, NilTimestampErr, ParseTimestampErr};

#[derive(Debug, Serialize, Deserialize)]
struct CompletedMatch {
    id: i32,
    red: (String, String),
    blue: (String, String),
    red_score: i32,
    blue_score: i32,
    round: i32,
    delta_elo: Option<f64>,
    started_timestamp: i64,
    event_id: i32
}

#[derive(Debug)]
struct FutureMatch {
    id: i32,
    red: (i32, i32),
    blue: (i32, i32),
    red_pred_score: Option<i32>,
    blue_pred_score: Option<i32>,
    round: i32,
    event_id: i32,
}

impl CompletedMatch {
    fn new() -> Self {
        CompletedMatch { id: 0, red: ("".into(), "".into()), blue: ("".into(), "".into()), red_score: 0, blue_score: 0, delta_elo: None, started_timestamp: 0, event_id: 0, round: 0 }
    }
}

struct MatchList {
    matches: Vec<CompletedMatch>
}

enum CompletedMatchError {
    NilTimestampErr,
    ParseTimestampErr,
    MatchNotScored
}

impl TryFrom<&Match> for CompletedMatch {
    type Error = CompletedMatchError;

    fn try_from(value: &Match) -> Result<Self, Self::Error> {
        let Match{id, started, alliances, event, round, .. } = value;

        if alliances.len() == 2 {
            let mut completed_match: CompletedMatch = CompletedMatch::new();
            completed_match.id = *id;
            completed_match.event_id = event.id;
            completed_match.round = *round;

            match started {
                Some(started_str) => {
                    match DateTime::parse_from_rfc3339(&*started_str) {
                        Ok(time) => {
                            completed_match.started_timestamp = time.timestamp();

                            let blue_alliance = alliances.get(0).unwrap();
                            let red_alliance = alliances.get(1).unwrap();

                            completed_match.blue_score = blue_alliance.score;
                            completed_match.red_score = red_alliance.score;

                            let default_team: AllianceTeam = AllianceTeam { team: IdInfo {id: 0, name: "0A".to_string(), code:None}, sitting: false };

                            completed_match.blue = (blue_alliance.teams.get(0).unwrap_or(&default_team).team.name.clone(), blue_alliance.teams.get(1).unwrap_or(&default_team).team.name.clone());
                            completed_match.red = (red_alliance.teams.get(0).unwrap_or(&default_team).team.name.clone(), red_alliance.teams.get(1).unwrap_or(&default_team).team.name.clone());

                            Ok(completed_match)
                        }
                        Err(parse_error) => {
                            println!("Error parsing timestamp: {}", parse_error);
                            Err(ParseTimestampErr)
                        }
                    }
                }
                None => {
                    // println!("No timestamp");
                    Err(NilTimestampErr)
                }

            }
        } else {
            println!("Match not scored");
            Err(MatchNotScored)
        }
    }
}

async fn get_matchlist(robot_events: &RobotEvents, event_id: i32, division_id: i32) -> Vec<CompletedMatch> {
    let mut matches: Vec<CompletedMatch> = vec!();

    match robot_events.event_division_matches(event_id, division_id, DivisionMatchesQuery::new().per_page(100)).await {
        Ok(match_list) => {

            for i in match_list.data.iter() {
                match CompletedMatch::try_from(i) {
                    Ok(completed_match) => {
                        matches.push(completed_match);
                    }
                    Err(_) => {}
                }
            }

            for i in 2..=match_list.meta.last_page {
                let new_response = robot_events.event_division_matches(event_id, division_id, DivisionMatchesQuery::new().per_page(100).page(i)).await;
                match new_response {
                    Ok(new_response_data) => {
                        for i in new_response_data.data {
                        match CompletedMatch::try_from(&i) {
                            Ok(completed_match) => {
                                matches.push(completed_match);
                            }
                            Err(_) => {}
                        }
                    }}
                    Err(error) => {
                        println!("Unable to get matches: {}", error);
                    }
                }
            }
        }
        Err(error) => {
            println!("Couldn't get matches: {}", error);
        }
    }

    matches
}

#[tokio::main]
async fn main() {
    let robot_events = RobotEvents::new("eyJ0eXAiOiJKV1QiLCJhbGciOiJSUzI1NiJ9.eyJhdWQiOiIzIiwianRpIjoiNGU4YzY4MmI4NGI5YTNhOGQ0MWEzYjE5Zjc3N2NhNGJlYWM4ZjgzYmJkZDNmNGFiY2M4Y2RkYmY0N2E0MDFlY2QyOTFiMjQ4ZTQ4ZDI3MGQiLCJpYXQiOjE3MDQzNDU4NjYuOTkxMTQ2MSwibmJmIjoxNzA0MzQ1ODY2Ljk5MTE0ODksImV4cCI6MjY1MTExNzA2Ni45ODY0MDY4LCJzdWIiOiIxMTkxMjEiLCJzY29wZXMiOltdfQ.hPfXL7wWYSolddNnVryNMjJRqqcEZHMJTt5P5-_ExRqmt_gWS7eiPn_oxScATZDB_ZzEwMFvSem4ZSVq1cm1PVI-ukxzRXvtLz-SQlzqAknttpamgDjxais9U1KfVihWHxReUfonX7sfMGVcDKCaZjLmOIecGi4uGBJMXim_aJRt-h4hFuosoaLO_ZDnqJp7BHv98k2yPFogaXVAKC5Bz04U1up-1Vcsu6JbRlRTdEIQa5qkgkPpUc_eXR3ySwmfpg9_3NO5SzqNl4iZ2Fin7EFUnIslcm-fEIpCeLdzz55wiDGNsgSKbl7vDTjLSMKm1C4B4JXmJWSurKRc4BH4FaiUD-iqoIYmVxBMJqjnPjwcYjVMd3lR22-DdjMlX0qEhVLDiUP9Be0HM2tMpzjSAA9DJPJtcA6WdXxysWoZuwlpyBq0AqtXYLMMeDCuRsInA5aS0hHGEwlY6bQSKEuc_Z6eH1v9GF8BOWnUhdajnazVEq2k7EVK-2QBvlcLQNepdFU1NbgdnkF2Uce3HGo7UHB3UrCdO8BBK8W_EX5yXeZGYRWS46NhxLpNvI7RqQLigVZlVOCdKXhIKdV5PdI1v3dhLpR8rz5cp9y9Nu9i5bd1BC9hlZkLNdCFsgheuJLY2YvYi_eP7Hr7MHtOlg2xTtoX8vSbXp0siw0xkd_pSv4");

    // let mut matches: Vec<CompletedMatch> = vec!();
    let mut teams_elo : HashMap<String, f64> = HashMap::new();

    let mut competitions = [51818, 51785, 51910, 51895, 51786, 51825, 51900, 51754, 51961, 51962, 52009, 51787, 52045, 52046, 51524, 51637, 52236, 52238, 51625, 51908, 52274, 52050, 52051, 51788, 52961, 51902, 52119, 52276, 52229, 51927, 53120, 53121, 53129, 51817, 52609, 51640, 51899, 51963, 52028, 52281, 52037, 52090, 52307, 52557, 52561, 53361, 51942, 52451, 52452, 53664, 53665, 51582, 51583, 51904, 51939, 52088, 52208, 52283, 52456, 52934, 52779, 51761, 52038, 52066, 52168, 52209, 52225, 52752, 53150, 53450, 53507, 53667, 52011, 52391, 52108, 52365, 52147, 52181, 52364, 52389, 52820, 53015, 53495, 52823, 51748, 53375, 53376, 54063, 52647, 53282, 53447, 51559, 51629, 51684, 51736, 51827, 52024, 52171, 52556, 52394, 53605, 51678, 53017, 53404, 52401, 53153, 53603, 53604, 51872, 52118, 51571, 51588, 51878, 51938, 51951, 52159, 52233, 52570, 52638, 52775, 52815, 52883, 53002, 53292, 53380, 53438, 53754, 53967, 53258, 53442, 53405, 52137, 53035, 54165, 53152, 53354, 51482, 53547, 53816, 53828, 51549, 51679, 51689, 51753, 51840, 51905, 51983, 52122, 52319, 52421, 52602, 52715, 52732, 52863, 52871, 52964, 52995, 53034, 53039, 53045, 53098, 53265, 53341, 53568, 53577, 54349, 51752, 52559, 52149, 52485, 52494, 54259, 54602, 51873, 52309, 51634, 51644, 51651, 51710, 51724, 51765, 51835, 51874, 52005, 52072, 52075, 52093, 52114, 52373, 52516, 52517, 52661, 52754, 52767, 52855, 52874, 52929, 52987, 53088, 53109, 53116, 53259, 53278, 53439, 53443, 53506, 53616, 53662, 53970, 51657, 51819, 51966, 52150, 52495, 51537, 52491, 52100, 53355, 51879, 52992, 54201, 54202, 51607, 51647, 51746, 51906, 51921, 51926, 51932, 51934, 51944, 51955, 52002, 52006, 52035, 52073, 52115, 52151, 52224, 52290, 52466, 52469, 52514, 52560, 52643, 52739, 52750, 52792, 52794, 52886, 52924, 52926, 52988, 53032, 53044, 53174, 53198, 53205, 53209, 53228, 53243, 53463, 53537, 53579, 53589, 53612, 53821, 53859, 54493, 54494, 51954, 53876, 53877, 52099, 52574, 54625, 52031, 53980, 54345, 54896, 51481, 51580, 51596, 51630, 51681, 51687, 51806, 51935, 51945, 52034, 52228, 52248, 52579, 52627, 52628, 52898, 52967, 52970, 52972, 53005, 53016, 53210, 53271, 53280, 53287, 53300, 53386, 53397, 53483, 53491, 53571, 53608, 53751, 53768, 54368, 54522, 51565, 51807, 52374, 52590, 53301, 53360, 51483, 52858, 51584, 51615, 51712, 52152, 52277, 51604, 51655, 51709, 51711, 51715, 51729, 51745, 51968, 52007, 52206, 52220, 52314, 52324, 52347, 52372, 52380, 52431, 52432, 52493, 52523, 52524, 52532, 52703, 52759, 52862, 52891, 53050, 53177, 53180, 53270, 53297, 53302, 53322, 53358, 53428, 53472, 53580, 53585, 53653, 53731, 53797, 53860, 53928, 53962, 53998, 54025, 54275, 54395, 54456, 54517, 54616, 51923, 53073, 53303, 54421, 54654, 53986, 55095, 51586, 51727, 51743, 52639, 53976, 55019, 52342, 53111, 53444, 52866, 53674, 53926, 51484, 54320, 51716, 52631, 53834, 54378, 54825, 51577, 51587, 51595, 51608, 51614, 51717, 51858, 51877, 51907, 51919, 51936, 52054, 52063, 52077, 52097, 52098, 52109, 52110, 52125, 52205, 52270, 52395, 52412, 52438, 52440, 52518, 52519, 52591, 52612, 52679, 52720, 52733, 52859, 52870, 52910, 52913, 52955, 52989, 53203, 53207, 53247, 53277, 53338, 53362, 53387, 53422, 53440, 53524, 53567, 53654, 53760, 53773, 53829, 53882, 53912, 54060, 54290, 54306, 54318, 54353, 54397, 54630, 54927, 52079, 52413, 52593, 53008, 53923, 54698, 54938, 54958, 51593, 54933, 51548, 54918, 54919, 52777, 53071, 53127, 53139, 53274, 54120, 51602, 51606, 51646, 51652, 51751, 51811, 51829, 51917, 51948, 51989, 51996, 52065, 52123, 52165, 52180, 52197, 52204, 52429, 52507, 52596, 52603, 52658, 52664, 52704, 52738, 52745, 52864, 52902, 52938, 53006, 53022, 53033, 53047, 53095, 53142, 53159, 53264, 53340, 53401, 53573, 53575, 53615, 53659, 53698, 53710, 53720, 53767, 53770, 53776, 53855, 53865, 54062, 54101, 54112, 54162, 54365, 54433, 54537, 51773, 53128, 53496, 54575, 54794, 54914, 52133, 53408, 55114, 51485, 51486, 51546, 51767, 51770, 51880, 51890, 52131, 53181, 53359, 54558, 54754, 51578, 51585, 51617, 51704, 51793, 51794, 51826, 51937, 51976, 52020, 52111, 52129, 52291, 52346, 52357, 52366, 52527, 52558, 52562, 52563, 52586, 52587, 52620, 52832, 52879, 52981, 53009, 53061, 53087, 53204, 53216, 53330, 53393, 53471, 53476, 53717, 53788, 53978, 53985, 54044, 54188, 54281, 54338, 54420, 54541, 54614, 54684, 54708, 54868, 55146, 51820, 51977, 52021, 55197, 54234, 53780, 55012, 55013, 55056, 51487, 51538, 51750, 52462, 55224];

    // let response = robot_events.events(EventsQuery::new().season(181).per_page(100).end("2024-01-04T04:02:16".to_string())).await.unwrap();
    //
    // for i in response.data {
    //     // println!("{}", i.name);
    //     competitions.push(i.id);
    // }
    //
    // for i in 2..=response.meta.last_page {
    //     let new_response = robot_events.events(EventsQuery::new().season(181).per_page(100).end("2024-01-04T04:02:16".to_string()).page(i)).await;
    //
    //     for i in new_response.unwrap().data {
    //         // println!("{}", i.name);
    //         competitions.push(i.id);
    //     }
    // }

    // println!("{:?}", competitions);
    // println!("{}", competitions.len());

    // for (index, i) in competitions.iter().enumerate() {
    //     matches.append(&mut get_matchlist(&robot_events, *i, 1).await);
    //
    //     async_std::task::sleep( Duration::from_millis( 5000 ) ).await;
    //
    //     println!("size: {}, i: {}, total: {}", matches.len(), index, competitions.len());
    // }

    // Specify the path to your JSON file
    let file_path = "match_data2.json";

    // Open the file in read-only mode
    let file = File::open(file_path).expect("Failed to open file");

    // Create a buffered reader for efficiency
    let mut reader = std::io::BufReader::new(file);

    // Read the contents of the file into a string
    let mut contents = String::new();
    reader.read_to_string(&mut contents).expect("Failed to read file");

    // Deserialize the JSON string into your struct
    let mut matches : Vec<CompletedMatch> = serde_json::from_str(&contents).expect("Failed to parse JSON");

    matches.sort_by(|a, b| a.started_timestamp.cmp(&b.started_timestamp));

    let mut total_match_score: i64 = 0;

    for CompletedMatch{red_score, blue_score, ..} in matches.iter() {
        total_match_score += ((red_score + blue_score) as i64).abs();
    }

    let mean = (total_match_score as f64)/((matches.len()*2) as f64);

    let mut variance = 0.0;

    for CompletedMatch{red_score, blue_score, ..} in matches.iter() {
        variance += (((*red_score - *blue_score) as f64)-mean).powf(2.0);
    }

    variance /= (matches.len()) as f64;
    let standard_deviation = variance.powf(0.5);

    println!("mean: {} var: {} std: {}", mean, variance, standard_deviation);

    for CompletedMatch{red_score, blue_score, red: (red1, red2), blue: (blue1, blue2), delta_elo, round, ..} in matches.iter_mut() {
        let red1_elo = match teams_elo.get(red1) {
            None => {mean/2.0}
            Some(elo) => {*elo}
        };
        let red2_elo = match teams_elo.get(red2) {
            None => {mean/2.0}
            Some(elo) => {*elo}
        };
        let blue1_elo = match teams_elo.get(blue1) {
            None => {mean/2.0}
            Some(elo) => {*elo}
        };
        let blue2_elo = match teams_elo.get(blue2) {
            None => {mean/2.0}
            Some(elo) => {*elo}
        };

        let red_rating = red1_elo + red2_elo;
        let blue_rating = blue1_elo + blue2_elo;

        let predicted_score_margin = red_rating - blue_rating;
        let actual_score_margin = (*red_score - *blue_score) as f64;
        let k = 72.0/250.0;

        let elo_change = k * (actual_score_margin - predicted_score_margin);
        *delta_elo = Some(elo_change);

        teams_elo.insert(red1.clone(), red1_elo + elo_change);
        teams_elo.insert(red2.clone(), red2_elo + elo_change);
        teams_elo.insert(blue1.clone(), blue1_elo - elo_change);
        teams_elo.insert(blue2.clone(), blue2_elo - elo_change);

        // println!("Elo_change: {}", elo_change);
        // println!("Red: ({}, {}), ({}, {})", red1, red1_elo, red2, red2_elo);
        // println!("Blue: ({}, {}), ({}, {})", blue1, blue1_elo, blue2, blue2_elo);
    }

    // println!("{:?}", matches)

    println!("Total matches: {}", matches.len());
    // for (key, value) in &teams_elo {
    //     println!("{},{}", key, value);
    // }

    let match_data = serde_json::to_string(&matches);

    let mut file = File::create("match_data.json");
    file.expect("REASON").write_all(match_data.expect("REASON").as_bytes());

    let team_data = serde_json::to_string(&teams_elo);

    let mut teams = File::create("team_data.json");
    teams.expect("REASON").write_all(team_data.expect("REASON").as_bytes());

    println!("Data saved");
}
