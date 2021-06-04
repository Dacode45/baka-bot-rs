#[macro_use]
extern crate lazy_static;

extern crate pretty_env_logger;
#[macro_use]
extern crate log;

use gpt3::CompletionResponse;
use lazy_static::lazy_static;
use rand::prelude::SliceRandom;
use regex::Regex;
use serde::Deserialize;
use serde_json::json;
use serenity::{
    async_trait,
    builder::CreateApplicationCommand,
    client::bridge::gateway::GatewayIntents,
    http::CacheHttp,
    model::{
        gateway::Ready,
        interactions::{ApplicationCommand, Interaction, InteractionResponseType},
    },
    prelude::*,
};
use std::collections::HashMap;
use std::env;
use std::sync::Mutex;

mod big_data;
mod gpt3;

#[derive(Debug, Deserialize)]
struct WordCount {
    word: String,
    syllables: u32,
}

const SYLLABLE_COUNTS_DATA: &'static str = include_str!("syllable_counts_data.csv");

lazy_static! {
    static ref SYLLABLE_COUNTS: Vec<WordCount> = {
        let mut rdr = csv::Reader::from_reader(SYLLABLE_COUNTS_DATA.as_bytes());
        rdr.deserialize()
            .map(|result: Result<WordCount, _>| result.unwrap())
            .filter(|w| w.syllables <= 5)
            .collect()
    };
    static ref COUNT_TO_WORDS: HashMap<u32, Vec<String>> = {
        let mut m: HashMap<u32, Vec<String>> = HashMap::new();
        for wc in SYLLABLE_COUNTS.iter() {
            m.entry(wc.syllables)
                .and_modify(|e| e.push(wc.word.clone()))
                .or_insert_with(|| vec![wc.word.clone()]);
        }
        m
    };
    static ref WORD_TO_COUNT: HashMap<String, u32> = SYLLABLE_COUNTS
        .iter()
        .map(|word| (word.word.clone(), word.syllables))
        .collect();
}

async fn gpt3_baka(client: &gpt3::Client) -> Result<Vec<String>, reqwest::Error> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r#"Baka: ([^.]+)\."#).unwrap();
        static ref baka_req: gpt3::CompletionRequest = gpt3::CompletionRequest {
            prompt: big_data::BAKA_PROMPT.to_string(),
            max_tokens: 64,
        };
    }
    let mut out = Vec::new();
    loop {
        let res = client.completion(&baka_req).await?;
        dbg!(&res);
        for cap in RE.captures_iter(&res.choices[0].text) {
            let words: Vec<&str> = cap[1].split(" ").map(|word| word.trim()).collect();
            let syllables = words.iter().fold(Some(0), |acc, word| {
                acc.and_then(|current| {
                    WORD_TO_COUNT
                        .get(&word.to_lowercase())
                        .map(|count| count + current)
                })
            });
            dbg!(&words);
            dbg!(&syllables);
            match syllables {
                Some(5) => out.push(words.join(" ")),
                _ => {}
            }
        }
        if out.len() > 0 {
            break;
        }
    }
    Ok(out)
}

fn gen_baka(mut target: u32) -> Vec<&'static str> {
    use rand::Rng;
    let mut rng = rand::thread_rng();

    let mut out = Vec::new();

    loop {
        if target == 0 {
            return out;
        }
        let amt = rng.gen_range(1..=target);
        let words = &COUNT_TO_WORDS[&amt];
        let word = words.choose(&mut rng).unwrap();
        out.push(word);
        target -= amt;
    }
}

struct Handler {
    pub gpt3: gpt3::Client,
}

impl Handler {
    pub fn new(client: gpt3::Client) -> Self {
        Handler { gpt3: client }
    }

    async fn dont_know(&self, ctx: Context, interaction: Interaction) {
        warn!("unknown command: {:?}", interaction);
        if let Some(channel) = interaction.channel_id {
            let _ = channel
                .say(&ctx.http, format!("I don't know what to do about that."))
                .await;
        }
    }
    async fn error(&self, ctx: Context, interaction: Interaction, err: String) {
        error!("error {}", &err);
        if let Some(channel) = interaction.channel_id {
            let _ = channel
                .say(
                    &ctx.http,
                    format!("I did my best, but something went wrong...Baka!"),
                )
                .await;
        }
    }
}

#[async_trait]
impl EventHandler for Handler {
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        let res = interaction
            .create_interaction_response(&ctx.http, |response| {
                response
                    .kind(InteractionResponseType::ChannelMessageWithSource)
                    .interaction_response_data(|message| message.content("Give me a second..."))
            })
            .await;
        match res {
            Err(e) => return self.error(ctx, interaction, format!("{}", e)).await,
            _ => {}
        }
        if let Some(data) = &interaction.data {
            match data.name.as_str() {
                "baka" => {
                    let res = gpt3_baka(&self.gpt3).await;
                    match res {
                        Ok(bakas) => {
                            let bakas: Vec<_> =
                                bakas.iter().map(|p| format!("Baka: {}.", p)).collect();
                            let bakas = bakas.join("\n");
                            if let Some(channel) = interaction.channel_id {
                                let res = channel.say(&ctx.http, &bakas).await;
                                debug!("result: {:?}", res);
                                info!("sent baka: '{}'", &bakas);
                            }
                        }
                        Err(e) => self.error(ctx, interaction, format!("{}", e)).await,
                    }
                }
                _ => self.dont_know(ctx, interaction).await,
            }
        } else {
            self.dont_know(ctx, interaction).await;
        }
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        info!("{} is connected!", ready.user.name);

        let baka_cmd = json!({
            "name": "test",
            "description": "testing baka bot"
        });

        let cmd = ctx
            .http
            .create_guild_application_command(494671450985201665, &baka_cmd)
            .await;

        info!("Registered guild command: {:?}", cmd);

        let cmd = ctx.http.create_global_application_command(&baka_cmd).await;

        info!("Registered global command: {:?}", cmd);

        let interactions = ApplicationCommand::get_global_application_commands(&ctx.http).await;

        info!(
            "I have the following global slash command(s): {:?}",
            interactions
        );
    }
}

#[tokio::main]
async fn main() {
    // Configure the client with your Discord bot token in the environment.
    pretty_env_logger::init();
    trace!("trace enabled");
    debug!("debug enabled");
    info!("info enabled");
    warn!("warn enabled");
    error!("error enabled");
    dotenv::dotenv().expect("Failed to read .env file");
    let token = env::var("DISCORD_TOKEN").expect("Expected a DISCORD_TOKEN in the environment");
    let gpt3_token = env::var("OPENAI_KEY").expect("Expected a OPENAI_KEY in the environment");

    let gpt3_client = gpt3::Client::new(gpt3_token);
    // The Application Id is usually the Bot User Id.
    let application_id: u64 = env::var("APPLICATION_ID")
        .expect("Expected an application id in the environment")
        .parse()
        .expect("application id is not a valid id");

    // Build our client.
    let mut client = Client::builder(token)
        .event_handler(Handler::new(gpt3_client))
        .application_id(application_id)
        .await
        .expect("Error creating client");

    // Finally, start a single shard, and start listening to events.
    //
    // Shards will automatically attempt to reconnect, and will perform
    // exponential backoff until it reconnects.
    if let Err(why) = client.start().await {
        error!("Client error: {:?}", why);
    }
}
