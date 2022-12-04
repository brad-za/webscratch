#![allow(dead_code, unused_variables, unused_imports)]
use csv::{Position, Reader};
use futures::executor::block_on;
use rand::Rng;
use scraper::{Html, Selector};
use select::document::Document;
use select::predicate::{Attr, Predicate};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt;
use std::fs::File;
use std::io::prelude::*;
use std::io::{self, BufRead, BufReader};
use std::{thread, time, usize};
#[tokio::main]
async fn main() {
    let player_list: Vec<Player> =
        read_csv("player_data.csv".to_string()).expect("failed to read csv");

    let url = "https://pokerdb.thehendonmob.com/ranking/all-time-money-list/";
    let proxies = read_proxies();
    let player_list = block_on(poker_players(url, &proxies, player_list));
    // let player_list = block_on(get_twitter(player_list, &proxies));

    write_csv(player_list).expect("failed to write csv");

    // // println!("{:#?}", player_list);
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Player {
    name: String,
    twitter: String,
    url: String,
    number: usize,
    page: usize,
}

impl Player {
    fn new(name: String, url: String, number: usize, page: usize) -> Player {
        Player {
            name,
            twitter: String::from(""),
            url,
            number,
            page,
        }
    }
}
#[derive(Debug, Clone)]
struct Proxy(String);

impl fmt::Display for Proxy {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

async fn poker_players(url: &str, prox: &Vec<Proxy>, player_list: Vec<Player>) -> Vec<Player> {
    let mut players: Vec<Player> = player_list;

    let mut player_count: usize = players.len();

    for page in (player_count / 100) + 1..(player_count / 100) + 10 {
        let prox_rng = &prox[rand::thread_rng().gen_range(1..prox.len())];

        let client = prox_client(prox_rng);

        let raw_res = client.get(format!("{url}/{page}")).send().await.unwrap();

        let status = raw_res.status();
        println!("page {} | status {}", &page, &status);
        let res = raw_res.text().await.unwrap();

        let doc = Document::from(res.as_str());

        for row in
            doc.find(Attr("class", "table table--ranking-list").descendant(Attr("class", "name")))
        {
            let name = row.text();
            let url = row
                .find(Attr("href", ()))
                .next()
                .unwrap()
                .attr("href")
                .unwrap();
            player_count += 1;
            players.push(Player::new(
                name.trim().to_string(),
                url.to_string(),
                player_count as usize,
                page as usize,
            ));

            // TODO: this is where new line need to be printed onto the csv
        }

        let players_on_current_page: Vec<&Player> =
            players.iter().filter(|p| p.page == page).collect();

        println!("{:#?}", players_on_current_page);
        println!("Page       | {}", page);
        println!("proxy used | {}", prox_rng);
        println!("status     | {}", status);
    }
    players
}

fn prox_client(prox_rng: &Proxy) -> reqwest::Client {
    reqwest::Client::builder()
        .proxy(reqwest::Proxy::http(prox_rng.to_string()).unwrap())
        .build()
        .unwrap()
}

async fn get_twitter(player_list: Vec<Player>, prox: &Vec<Proxy>) -> Vec<Player> {
    let mut players: Vec<Player> = player_list;
    let mut player_req_count = 1; // each iter that makes a request will increment this var until 12
                                  // println!("{}", players.len());

    for mut player in &mut players {
        if player_req_count == 13 {
            // println!("sleeping");
            // let ten_millis = time::Duration::from_secs(1 * 60);

            // thread::sleep(ten_millis);
            // println!("done sleeping");
            // player_req_count = 14;
            break;
        } else if player.twitter.is_empty() {
            println!("player count is = {}", player_req_count);
            let prox_rng = &prox[rand::thread_rng().gen_range(1..prox.len())];
            let player_url = format!("https://pokerdb.thehendonmob.com{}", player.url);
            let client = prox_client(prox_rng);

            let raw_res = client.get(player_url).send().await.unwrap();
            println!("status {}", raw_res.status());
            println!("prox {}", prox_rng);

            let html = raw_res.text().await.unwrap();

            let document = Html::parse_document(&html);

            let selector = Selector::parse("div.twitter").unwrap();
            let link_selector = Selector::parse("a.twitter-follow-button").unwrap();
            let twitter_button_iframe = document.select(&selector).next();
            match twitter_button_iframe {
                Some(v) => {
                    let twitter_link = v
                        .select(&link_selector)
                        .next()
                        .unwrap()
                        .value()
                        .attr("href")
                        .unwrap();
                    player.twitter = twitter_link.to_string();
                    player_req_count += 1;

                    println!("{:#?}", player);
                    println!("{}", prox_rng);
                    // TODO: print line to csv like a chad
                }
                None => {
                    player.twitter = "NA".to_string();
                    player_req_count += 1;
                    println!("{:#?}", player);
                }
            }
        }
    }

    players
}

fn read_proxies() -> Vec<Proxy> {
    let file = File::open("http_proxies_2.txt").expect("file not found!");
    let reader = BufReader::new(file);
    let mut proxies: Vec<Proxy> = vec![];
    for line in reader.lines() {
        // println!("{:#?}", &line.unwrap());
        proxies.push(Proxy(line.unwrap()));
    }
    // println!("{}", proxies[1]);
    proxies
}

fn write_csv(player_list: Vec<Player>) -> Result<(), Box<dyn Error>> {
    // TODO: get player list from poker_players fn and write to csv.
    // TODO: then do the twitter links reading from csv

    let mut writer = csv::Writer::from_path("player_data.csv")?;

    for player in player_list {
        writer.serialize(Player {
            name: player.name,
            twitter: player.twitter,
            url: player.url,
            number: player.number,
            page: player.page,
        })?
    }

    Ok(())
}

fn read_csv(path: String) -> Result<Vec<Player>, Box<dyn Error>> {
    let mut player_list: Vec<Player> = Vec::new();
    let mut reader = csv::Reader::from_path(path)?;

    for result in reader.deserialize() {
        let player: Player = result?;
        player_list.push(player);
    }

    // println!("{:?}", player_list);

    Ok(player_list)
}

async fn get_pages(url: String) -> i32 {
    let raw_res = reqwest::get(url).await.unwrap();
    if raw_res.status() == 200 {
        println!("Pages collected");
    }
    let res = raw_res.text().await.unwrap();
    let doc = Document::from(res.as_str());
    let mut pages: i32 = 0;
    for val in doc.find(Attr("onchange", "var link='/ranking/all-time-money-list/%s'; window.location=link.replace('%s', this.value);").descendant(Attr("value", ()))) {
        pages = val.text().parse().unwrap();
    }
    pages
}
