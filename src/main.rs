use std::collections::HashMap;
use serde_json;
use rand::Rng;
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use rayon::prelude::*;

#[derive(Clone, PartialEq)]
struct Player {
    name: String,
    serve_win_prob: f64,
    ace_prob: f64,
    double_fault_prob: f64,
}

struct TennisMatch {
    player1: Player,
    player2: Player,
    best_of: i32,
    grand_slam: bool,
    server: Option<Player>,
    receiver: Option<Player>,
    score: HashMap<String, Vec<i32>>,
    set_history: Vec<HashMap<String, HashMap<String, i32>>>,
    total_shots: i32,
    point_log: Vec<HashMap<String, serde_json::Value>>,
    stats: HashMap<String, HashMap<String, i32>>,
    last_point_winner: Option<Player>,
    consecutive_points: i32,
    last_point_ace: bool,
    is_tiebreak: bool,
    tiebreak_points: i32,
    tiebreak_server: Option<Player>,
}

impl TennisMatch {
    fn new(player1: Player, player2: Player, best_of: i32, grand_slam: bool) -> Self {
        let mut score = HashMap::new();
        score.insert("sets".to_string(), vec![0, 0]);
        score.insert("games".to_string(), vec![0, 0]);
        score.insert("points".to_string(), vec![0, 0]);

        let mut stats = HashMap::new();
        stats.insert(player1.name.clone(), HashMap::new());
        stats.insert(player2.name.clone(), HashMap::new());
        stats.get_mut(&player1.name).unwrap().insert("aces".to_string(), 0);
        stats.get_mut(&player1.name).unwrap().insert("double_faults".to_string(), 0);
        stats.get_mut(&player2.name).unwrap().insert("aces".to_string(), 0);
        stats.get_mut(&player2.name).unwrap().insert("double_faults".to_string(), 0);

        TennisMatch {
            player1,
            player2,
            best_of,
            grand_slam,
            server: None,
            receiver: None,
            score,
            set_history: Vec::new(),
            total_shots: 0,
            point_log: Vec::new(),
            stats,
            last_point_winner: None,
            consecutive_points: 0,
            last_point_ace: false,
            is_tiebreak: false,
            tiebreak_points: 0,
            tiebreak_server: None,
        }
    }

    fn switch_server(&mut self) {
        std::mem::swap(&mut self.server, &mut self.receiver);
    }

    fn is_final_set(&self) -> bool {
        self.score["sets"].iter().sum::<i32>() == self.best_of - 1
    }

    fn is_set_over(&self) -> bool {
        if !self.is_tiebreak {
            self.score["games"].iter().max().unwrap() >= &6 && (self.score["games"][0] - self.score["games"][1]).abs() >= 2
        } else {
            if self.grand_slam && self.is_final_set() {
                self.score["points"].iter().max().unwrap() >= &10 && (self.score["points"][0] - self.score["points"][1]).abs() >= 2
            } else {
                self.score["points"].iter().max().unwrap() >= &7 && (self.score["points"][0] - self.score["points"][1]).abs() >= 2
            }
        }
    }

    fn format_point_score(&self) -> String {
        if !self.is_tiebreak {
            let server_points = self.score["points"][if self.server.as_ref().unwrap().name == self.player1.name { 0 } else { 1 }];
            let receiver_points = self.score["points"][if self.server.as_ref().unwrap().name == self.player1.name { 1 } else { 0 }];
            if server_points == receiver_points && server_points >= 3 {
                "Deuce".to_string()
            } else if server_points.max(receiver_points) >= 4 {
                if (server_points - receiver_points).abs() == 1 {
                    if server_points > receiver_points { "Ad-In".to_string() } else { "Ad-Out".to_string() }
                } else if (server_points - receiver_points).abs() >= 2 {
                    "GAME".to_string()
                } else {
                    format!("{}-{}", self.point_to_tennis_score(server_points), self.point_to_tennis_score(receiver_points))
                }
            } else {
                format!("{}-{}", self.point_to_tennis_score(server_points), self.point_to_tennis_score(receiver_points))
            }
        } else {
            let server_points = self.score["points"][if self.server.as_ref().unwrap().name == self.player1.name { 0 } else { 1 }];
            let receiver_points = self.score["points"][if self.server.as_ref().unwrap().name == self.player1.name { 1 } else { 0 }];
            format!("{}-{}", server_points, receiver_points)
        }
    }

    fn point_to_tennis_score(&self, points: i32) -> String {
        if self.is_tiebreak {
            points.to_string()
        } else {
            match points {
                0 => "0".to_string(),
                1 => "15".to_string(),
                2 => "30".to_string(),
                3 => "40".to_string(),
                _ => points.to_string(),
            }
        }
    }

    fn format_game_score(&self) -> String {
        let server_games = self.score["games"][if self.server.as_ref().unwrap().name == self.player1.name { 0 } else { 1 }];
        let receiver_games = self.score["games"][if self.server.as_ref().unwrap().name == self.player1.name { 1 } else { 0 }];
        format!("{}-{}", server_games, receiver_games)
    }

    fn format_set_score(&self) -> String {
        let server_sets = self.score["sets"][if self.server.as_ref().unwrap().name == self.player1.name { 0 } else { 1 }];
        let receiver_sets = self.score["sets"][if self.server.as_ref().unwrap().name == self.player1.name { 1 } else { 0 }];
        format!("{}-{}", server_sets, receiver_sets)
    }

    fn log_point(&mut self) -> (bool, bool) {
        let point_score = self.format_point_score();
        let mut game_over = false;
        let mut set_over = false;

        if self.is_tiebreak {
            if self.is_set_over() {
                set_over = true;
                game_over = true;
                let winning_player_index = if self.score["points"][0] > self.score["points"][1] { 0 } else { 1 };
                self.score.get_mut("games").unwrap()[winning_player_index] += 1;
                self.score.get_mut("sets").unwrap()[winning_player_index] += 1;
                self.is_tiebreak = false;
            }
        } else {
            if point_score == "GAME" {
                game_over = true;
                let winning_player_index = if self.score["points"][0] > self.score["points"][1] { 0 } else { 1 };
                self.score.get_mut("games").unwrap()[winning_player_index] += 1;
            }

            if self.is_set_over() {
                set_over = true;
                let winning_player_index = if self.score["games"][0] > self.score["games"][1] { 0 } else { 1 };
                self.score.get_mut("sets").unwrap()[winning_player_index] += 1;
            } else if self.score["games"][0] == 6 && self.score["games"][1] == 6 {
                self.is_tiebreak = true;
                self.score.insert("points".to_string(), vec![0, 0]);
                self.tiebreak_server = self.server.clone();
                self.tiebreak_points = 0;
            }
        }

        let game_score = self.format_game_score();
        let set_score = self.format_set_score();

        // Calculate probabilities
        let match_win_prob1 = self.calculate_match_win_probability(&self.player1);
        let match_win_prob2 = self.calculate_match_win_probability(&self.player2);
        let set_win_prob1 = self.calculate_set_win_probability(&self.player1);
        let set_win_prob2 = self.calculate_set_win_probability(&self.player2);
        let game_win_prob1 = self.calculate_game_win_probability(&self.player1);
        let game_win_prob2 = self.calculate_game_win_probability(&self.player2);
        let next_point_prob1 = self.calculate_next_point_win_probability(&self.player1);
        let next_point_prob2 = self.calculate_next_point_win_probability(&self.player2);
        let ace_prob = self.calculate_ace_probability();
        let tiebreak_prob = self.calculate_tiebreak_probability();

        let mut point_info = HashMap::new();
        point_info.insert("server".to_string(), serde_json::Value::String(self.server.as_ref().unwrap().name.clone()));
        point_info.insert("receiver".to_string(), serde_json::Value::String(self.receiver.as_ref().unwrap().name.clone()));
        point_info.insert("point_score".to_string(), serde_json::Value::String(point_score));
        point_info.insert("game_score".to_string(), serde_json::Value::String(game_score));
        point_info.insert("set_score".to_string(), serde_json::Value::String(set_score));
        point_info.insert(format!("{}_match_win_prob", self.player1.name), serde_json::Value::Number(serde_json::Number::from_f64(match_win_prob1).unwrap()));
        point_info.insert(format!("{}_match_win_prob", self.player2.name), serde_json::Value::Number(serde_json::Number::from_f64(match_win_prob2).unwrap()));
        point_info.insert(format!("{}_set_win_prob", self.player1.name), serde_json::Value::Number(serde_json::Number::from_f64(set_win_prob1).unwrap()));
        point_info.insert(format!("{}_set_win_prob", self.player2.name), serde_json::Value::Number(serde_json::Number::from_f64(set_win_prob2).unwrap()));
        point_info.insert(format!("{}_game_win_prob", self.player1.name), serde_json::Value::Number(serde_json::Number::from_f64(game_win_prob1).unwrap()));
        point_info.insert(format!("{}_game_win_prob", self.player2.name), serde_json::Value::Number(serde_json::Number::from_f64(game_win_prob2).unwrap()));
        point_info.insert(format!("{}_next_point_win_prob", self.player1.name), serde_json::Value::Number(serde_json::Number::from_f64(next_point_prob1).unwrap()));
        point_info.insert(format!("{}_next_point_win_prob", self.player2.name), serde_json::Value::Number(serde_json::Number::from_f64(next_point_prob2).unwrap()));
        point_info.insert("next_serve_ace_prob".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(ace_prob).unwrap()));
        point_info.insert("tiebreak_prob".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(tiebreak_prob).unwrap()));

        self.point_log.push(point_info);

        (game_over, set_over)
    }

    fn play_point(&mut self) -> Player {
        self.total_shots += 1;
        let mut rng = rand::thread_rng();
        let ace_prob = self.calculate_ace_probability();

        let server_name = self.server.as_ref().unwrap().name.clone();
        let is_server_player1 = server_name == self.player1.name;

        let (winner, is_ace, is_double_fault) = if rng.gen::<f64>() < ace_prob {
            (self.server.as_ref().unwrap().clone(), true, false)
        } else if rng.gen::<f64>() < self.server.as_ref().unwrap().double_fault_prob {
            (self.receiver.as_ref().unwrap().clone(), false, true)
        } else if rng.gen::<f64>() < self.server.as_ref().unwrap().serve_win_prob {
            (self.server.as_ref().unwrap().clone(), false, false)
        } else {
            (self.receiver.as_ref().unwrap().clone(), false, false)
        };

        // Update stats
        if is_ace {
            *self.stats.get_mut(&server_name).unwrap().entry("aces".to_string()).or_insert(0) += 1;
        }
        if is_double_fault {
            *self.stats.get_mut(&server_name).unwrap().entry("double_faults".to_string()).or_insert(0) += 1;
        }

        // Update score
        if winner.name == server_name {
            self.score.get_mut("points").unwrap()[if is_server_player1 { 0 } else { 1 }] += 1;
        } else {
            self.score.get_mut("points").unwrap()[if is_server_player1 { 1 } else { 0 }] += 1;
        }

        self.last_point_ace = is_ace;

        if Some(&winner) == self.last_point_winner.as_ref() {
            self.consecutive_points += 1;
        } else {
            self.consecutive_points = 1;
        }
        self.last_point_winner = Some(winner.clone());

        if self.is_tiebreak {
            self.tiebreak_points += 1;
            if self.tiebreak_points % 2 == 1 {
                self.switch_server();
            }
        }

        winner
    }

    fn play_game(&mut self) -> (Player, bool) {
        if !self.is_tiebreak {
            self.score.insert("points".to_string(), vec![0, 0]);
        }
        self.last_point_winner = None;
        self.consecutive_points = 0;
        self.last_point_ace = false;
        self.stats.get_mut(&self.server.as_ref().unwrap().name).unwrap().insert("aces".to_string(), 0);
        self.stats.get_mut(&self.server.as_ref().unwrap().name).unwrap().insert("double_faults".to_string(), 0);

        loop {
            let winner = self.play_point();
            let (game_over, set_over) = self.log_point();
            if game_over || set_over {
                if !set_over && !self.is_tiebreak {
                    self.switch_server();
                }
                return (winner, set_over);
            }
        }
    }

    fn play_set(&mut self) -> Player {
        let mut set_stats = HashMap::new();
        set_stats.insert(self.player1.name.clone(), HashMap::new());
        set_stats.insert(self.player2.name.clone(), HashMap::new());

        loop {
            let (winner, set_over) = self.play_game();
            if set_over {
                for player_name in [&self.player1.name, &self.player2.name].iter() {
                    let aces = *self.stats.get(*player_name).unwrap().get("aces").unwrap_or(&0);
                    let double_faults = *self.stats.get(*player_name).unwrap().get("double_faults").unwrap_or(&0);
                    
                    set_stats.get_mut(*player_name).unwrap().insert("aces".to_string(), aces);
                    set_stats.get_mut(*player_name).unwrap().insert("double_faults".to_string(), double_faults);
                    
                    self.stats.get_mut(*player_name).unwrap().insert("aces".to_string(), 0);
                    self.stats.get_mut(*player_name).unwrap().insert("double_faults".to_string(), 0);
                }
                self.set_history.push(set_stats);
                self.score.insert("games".to_string(), vec![0, 0]);
                self.score.insert("points".to_string(), vec![0, 0]);
                self.is_tiebreak = false;
                self.tiebreak_points = 0;
                self.switch_server();
                return winner;
            }
        }
    }

    fn play_match(&mut self) -> Player {
        let mut rng = rand::thread_rng();
        self.server = Some(if rng.gen::<bool>() { self.player1.clone() } else { self.player2.clone() });
        self.receiver = Some(if self.server.as_ref().unwrap().name == self.player1.name { self.player2.clone() } else { self.player1.clone() });

        while self.score["sets"].iter().max().unwrap() < &((self.best_of / 2) + 1) {
            let _set_winner = self.play_set();
        }

        if self.score["sets"][0] > self.score["sets"][1] { self.player1.clone() } else { self.player2.clone() }
    }

    fn calculate_match_win_probability(&self, player: &Player) -> f64 {
 
        let player_sets = self.score["sets"][if player.name == self.player1.name { 0 } else { 1 }];
        let opponent_sets = self.score["sets"][if player.name == self.player1.name { 1 } else { 0 }];
        let player_games = self.score["games"][if player.name == self.player1.name { 0 } else { 1 }];
        let opponent_games = self.score["games"][if player.name == self.player1.name { 1 } else { 0 }];

        let base_prob = 0.5 + (player_sets - opponent_sets) as f64 * 0.1;
        let game_adjustment = (player_games - opponent_games) as f64 * 0.01;
        (base_prob + game_adjustment).max(0.0).min(1.0)
    }

    fn calculate_set_win_probability(&self, player: &Player) -> f64 {
        let player_games = self.score["games"][if player.name == self.player1.name { 0 } else { 1 }];
        let opponent_games = self.score["games"][if player.name == self.player1.name { 1 } else { 0 }];

        let base_prob = 0.5 + (player_games - opponent_games) as f64 * 0.05;
        base_prob.max(0.0).min(1.0)
    }

    fn calculate_game_win_probability(&self, player: &Player) -> f64 {
        let is_server = self.server.as_ref().unwrap().name == player.name;
        let player_points = self.score["points"][if is_server { 0 } else { 1 }];
        let opponent_points = self.score["points"][if is_server { 1 } else { 0 }];

        let base_prob = if is_server { self.server.as_ref().unwrap().serve_win_prob } else { 1.0 - self.server.as_ref().unwrap().serve_win_prob };
        let point_adjustment = (player_points - opponent_points) as f64 * 0.05;
        (base_prob + point_adjustment).max(0.0).min(1.0)
    }

    fn calculate_next_point_win_probability(&self, player: &Player) -> f64 {
        let base_prob = if self.server.as_ref().unwrap().name == player.name {
            self.server.as_ref().unwrap().serve_win_prob
        } else {
            1.0 - self.server.as_ref().unwrap().serve_win_prob
        };

        let score_diff = self.score["points"][0] - self.score["points"][1];
        let score_adjustment = if player.name == self.server.as_ref().unwrap().name {
            0.02 * score_diff as f64
        } else {
            -0.02 * score_diff as f64
        };

        let momentum_adjustment = if Some(player) == self.last_point_winner.as_ref() {
            (0.01 * self.consecutive_points as f64).min(0.05)
        } else if self.last_point_winner.is_some() {
            -(0.01 * self.consecutive_points as f64).min(0.05)
        } else {
            0.0
        };

        let recent_ace_adjustment = if self.stats[&player.name]["aces"] > 0 { 0.03 } else { 0.0 };
        let recent_df_adjustment = if self.stats[&player.name]["double_faults"] > 0 { -0.03 } else { 0.0 };

        (base_prob + score_adjustment + momentum_adjustment + recent_ace_adjustment + recent_df_adjustment).max(0.0).min(1.0)
    }

    fn calculate_ace_probability(&self) -> f64 {
        let base_prob = self.server.as_ref().unwrap().ace_prob;
        let score_diff = self.score["points"][0] - self.score["points"][1];
        let score_adjustment = 0.01 * score_diff as f64;

        let momentum_adjustment = if Some(self.server.as_ref().unwrap()) == self.last_point_winner.as_ref() {
            (0.005 * self.consecutive_points as f64).min(0.02)
        } else {
            0.0
        };

        let recent_ace_adjustment = if self.last_point_ace { 0.02 } else { 0.0 };

        (base_prob + score_adjustment + momentum_adjustment + recent_ace_adjustment).max(0.0).min(0.3)
    }

    fn calculate_tiebreak_probability(&self) -> f64 {
        let games_sum: i32 = self.score["games"].iter().sum();
        match games_sum {
            0..=9 => 0.1,
            10 => 0.2,
            11 => 0.5,
            _ => 1.0,
        }
    }
}

/*
fn simulate_single_match(player1: Player, player2: Player, best_of: i32, grand_slam: bool) -> (String, i32, Vec<HashMap<String, serde_json::Value>>, HashMap<String, i32>, HashMap<String, i32>) {
    let mut match_sim = TennisMatch::new(player1.clone(), player2.clone(), best_of, grand_slam);
    let winner = match_sim.play_match();
    let total_shots = match_sim.total_shots;
    let point_log = match_sim.point_log;

    let mut aces = HashMap::new();
    let mut double_faults = HashMap::new();

    for player_name in &[player1.name.as_str(), player2.name.as_str()] {
        aces.insert(player_name.to_string(), match_sim.set_history.iter()
            .map(|set_stats| *set_stats.get(*player_name).unwrap().get("aces").unwrap_or(&0))
            .sum());
        double_faults.insert(player_name.to_string(), match_sim.set_history.iter()
            .map(|set_stats| *set_stats.get(*player_name).unwrap().get("double_faults").unwrap_or(&0))
            .sum());
    }

    (winner.name, total_shots, point_log, aces, double_faults)
}
*/

fn simulate_batch(player1: Player, player2: Player, best_of: i32, grand_slam: bool, batch_size: usize, save_logs: bool, filename: &str) -> (HashMap<String, i32>, i32, HashMap<String, i32>, HashMap<String, i32>) {
    let mut match_wins = HashMap::new();
    match_wins.insert(player1.name.clone(), 0);
    match_wins.insert(player2.name.clone(), 0);
    let mut total_shots = 0;
    let mut all_point_logs = Vec::new();
    let mut total_aces = HashMap::new();
    total_aces.insert(player1.name.clone(), 0);
    total_aces.insert(player2.name.clone(), 0);
    let mut total_double_faults = HashMap::new();
    total_double_faults.insert(player1.name.clone(), 0);
    total_double_faults.insert(player2.name.clone(), 0);

    for _ in 0..batch_size {
        let mut match_sim = TennisMatch::new(player1.clone(), player2.clone(), best_of, grand_slam);
        let winner = match_sim.play_match();
        *match_wins.get_mut(&winner.name).unwrap() += 1;
        total_shots += match_sim.total_shots;
        all_point_logs.extend(match_sim.point_log);

        for player_name in &[player1.name.as_str(), player2.name.as_str()] {
            let aces_sum: i32 = match_sim.set_history.iter()
                .map(|set_stats| set_stats.get(*player_name)
                    .and_then(|player_stats| player_stats.get("aces"))
                    .unwrap_or(&0))
                .sum();
            *total_aces.get_mut(*player_name).unwrap() += aces_sum;

            let double_faults_sum: i32 = match_sim.set_history.iter()
                .map(|set_stats| set_stats.get(*player_name)
                    .and_then(|player_stats| player_stats.get("double_faults"))
                    .unwrap_or(&0))
                .sum();
            *total_double_faults.get_mut(*player_name).unwrap() += double_faults_sum;
        }
    }

    if save_logs {
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .append(true)
            .open(filename)
            .unwrap();

        if file.metadata().unwrap().len() == 0 {
            writeln!(file, "server,receiver,point_score,game_score,set_score,{0}_match_win_prob,{1}_match_win_prob,{0}_set_win_prob,{1}_set_win_prob,{0}_game_win_prob,{1}_game_win_prob,{0}_next_point_win_prob,{1}_next_point_win_prob,next_serve_ace_prob,tiebreak_prob",
                player1.name, player2.name)
                .unwrap();
        }

        for point in all_point_logs {
            writeln!(file, "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
                point["server"].as_str().unwrap_or(""),
                point["receiver"].as_str().unwrap_or(""),
                point["point_score"].as_str().unwrap_or(""),
                point["game_score"].as_str().unwrap_or(""),
                point["set_score"].as_str().unwrap_or(""),
                point[&format!("{}_match_win_prob", player1.name)].as_f64().unwrap_or(0.0),
                point[&format!("{}_match_win_prob", player2.name)].as_f64().unwrap_or(0.0),
                point[&format!("{}_set_win_prob", player1.name)].as_f64().unwrap_or(0.0),
                point[&format!("{}_set_win_prob", player2.name)].as_f64().unwrap_or(0.0),
                point[&format!("{}_game_win_prob", player1.name)].as_f64().unwrap_or(0.0),
                point[&format!("{}_game_win_prob", player2.name)].as_f64().unwrap_or(0.0),
                point[&format!("{}_next_point_win_prob", player1.name)].as_f64().unwrap_or(0.0),
                point[&format!("{}_next_point_win_prob", player2.name)].as_f64().unwrap_or(0.0),
                point["next_serve_ace_prob"].as_f64().unwrap_or(0.0),
                point["tiebreak_prob"].as_f64().unwrap_or(0.0)
            ).unwrap();
        }
    }

    (match_wins, total_shots, total_aces, total_double_faults)
}

fn simulate_match_parallel(player1: Player, player2: Player, best_of: i32, grand_slam: bool, num_simulations: usize, _max_workers: usize, batch_size: usize, log_interval: usize) -> (HashMap<String, i32>, i32, u128, HashMap<String, i32>, HashMap<String, i32>) {
    let match_wins = Arc::new(Mutex::new(HashMap::new()));
    match_wins.lock().unwrap().insert(player1.name.clone(), 0);
    match_wins.lock().unwrap().insert(player2.name.clone(), 0);
    let total_shots = Arc::new(Mutex::new(0));
    let total_aces = Arc::new(Mutex::new(HashMap::new()));
    total_aces.lock().unwrap().insert(player1.name.clone(), 0);
    total_aces.lock().unwrap().insert(player2.name.clone(), 0);
    let total_double_faults = Arc::new(Mutex::new(HashMap::new()));
    total_double_faults.lock().unwrap().insert(player1.name.clone(), 0);
    total_double_faults.lock().unwrap().insert(player2.name.clone(), 0);

    let start_time = Instant::now();

    (0..num_simulations / batch_size).into_par_iter().for_each(|i| {
        let save_logs = (i + 1) * batch_size % log_interval == 0;
        let (batch_match_wins, batch_shots, batch_aces, batch_double_faults) = simulate_batch(
            player1.clone(),
            player2.clone(),
            best_of,
            grand_slam,
            batch_size,
            save_logs,
            "match_log_parallel.csv",
        );

        let mut match_wins = match_wins.lock().unwrap();
        for (player, wins) in batch_match_wins {
            *match_wins.entry(player).or_insert(0) += wins;
        }
        drop(match_wins);

        *total_shots.lock().unwrap() += batch_shots;

        let mut total_aces = total_aces.lock().unwrap();
        for (player, aces) in batch_aces {
            *total_aces.entry(player).or_insert(0) += aces;
        }
        drop(total_aces);

        let mut total_double_faults = total_double_faults.lock().unwrap();
        for (player, dfs) in batch_double_faults {
            *total_double_faults.entry(player).or_insert(0) += dfs;
        }
        drop(total_double_faults);
    });

    let execution_time = start_time.elapsed().as_millis();

    // Safely unwrap the Arc<Mutex<_>> values
    let final_match_wins = Arc::try_unwrap(match_wins).unwrap().into_inner().unwrap();
    let final_total_shots = Arc::try_unwrap(total_shots).unwrap().into_inner().unwrap();
    let final_total_aces = Arc::try_unwrap(total_aces).unwrap().into_inner().unwrap();
    let final_total_double_faults = Arc::try_unwrap(total_double_faults).unwrap().into_inner().unwrap();

    (final_match_wins, final_total_shots, execution_time, final_total_aces, final_total_double_faults)
}

fn main() {
    let num_simulations = 10000;
    let num_sets = 5;
    let max_workers = 10;
    let batch_size = 10;
    let log_interval = 10000;

    let player1 = Player {
        name: "Federer".to_string(),
        serve_win_prob: 0.65,
        ace_prob: 0.10,
        double_fault_prob: 0.05,
    };

    let player2 = Player {
        name: "Nadal".to_string(),
        serve_win_prob: 0.62,
        ace_prob: 0.08,
        double_fault_prob: 0.04,
    };

    let (results, total_shots, execution_time, aces, double_faults) = simulate_match_parallel(
        player1.clone(),
        player2.clone(),
        num_sets,
        true,
        num_simulations,
        max_workers,
        batch_size,
        log_interval,
    );

    println!("Percentage of Match wins after {} matches:", num_simulations);
    for (player, wins) in &results {
        println!("{}: {:.2}%", player, (*wins as f64 / num_simulations as f64) * 100.0);
    }

    println!("\nTotal shots played: {}", total_shots);
    println!("Execution time: {:.2} milliseconds", execution_time);

    println!("\nMatch statistics:");
    for player in &[&player1, &player2] {
        println!("{}:", player.name);
        println!(" Avg. Aces per match: {:.2}", *aces.get(&player.name).unwrap_or(&0) as f64 / num_simulations as f64);
        println!(" Avg. Double faults per match: {:.2}", *double_faults.get(&player.name).unwrap_or(&0) as f64 / num_simulations as f64);
    }

    println!("\nPoint-by-point log exported to 'match_log_parallel.csv'");
}