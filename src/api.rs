//! Easy to use async riot api wrapper.
//!
//! # Introduction
//! This module contains all the things needed to talk with Riot API.
//! The most important type here is
//! [`LeagueClient`], as it is the main way of getting the data from API. See [`LeagueClient`] for more information.
use crate::ddragon::DDragonClient;
use crate::constants::{LanguageCode, RankedQueue, RankedTier, Region};
use crate::dto::api::{ChampionInfo, ChampionMastery, LeagueInfo, Summoner};
use crate::error::*;
use crate::types::{Cache, Client};
use crate::utils::{cached_resp, construct_hyper_client};
use futures::Future;

use hyper::{HeaderMap, Uri};
use snafu::ResultExt;

use log::{debug, trace};

use std::collections::HashMap;
use std::env;

use crate::constants::division::Division;
use std::str;
use std::sync::{Arc, Mutex};

/// Main type for calling League API Endpoints.
/// Instances of `LeagueClient` can be created using [`new`] with a [`Region`] parameter
///
/// `LeagueClient` can have an embedded [`DDragonClient`] instance embedded in itself,
/// reference to which can be obtained using [`ddragon`]. ***NOTE***: this method will panic if
/// you don't create the instance using [`with_ddragon`].
///
/// [`new`]: #method.new
/// [`Region`]: ../constants/region/struct.Region.html
/// [`DDragonClient`]: ../ddragon/struct.DDragonClient.html
/// [`ddragon`]: #method.ddragon
/// [`with_ddragon`]: #method.with_ddragon
#[derive(Debug)]
pub struct LeagueClient {
    client: Client,
    cache: Cache,
    region: Region,
    base_url: String,
    ddragon: Option<DDragonClient>,
    api_key: String,
}

impl LeagueClient {
    /// Constructor function for LeagueAPI struct, accepts type as a parameter
    ///
    /// # Panics
    /// This will panic if you do not provide the RIOT_API_KEY environment variable with value being api token.
    pub fn new(region: Region) -> Result<LeagueClient, ClientError> {
        let base_url = format!("https://{}.api.riotgames.com/lol", region.as_platform_str());
        let api_key = std::env::var("RIOT_API_KEY").context(NoToken {})?;
        check_token(&api_key)?;
        let client = construct_hyper_client();
        let cache: Cache = Arc::new(Mutex::new(HashMap::new()));
        Ok(LeagueClient {
            region,
            base_url,
            ddragon: None,
            cache,
            client,
            api_key,
        })
    }

    /// Adds an embedded ddragon client instance to league api client that shares cache and client with parent.
    pub fn with_ddragon(self, language: LanguageCode) -> Self {
        let ddragon =
            DDragonClient::new_for_lapi(self.client.clone(), self.cache.clone(), language);
        LeagueClient {
            ddragon: Some(ddragon),
            ..self
        }
    }

    /// Gets mutable (because of cache) reference to ddragon client embedded in lapi client.
    ///
    /// # Panics
    /// Do not call `ddragon` if [`with_ddragon`] is not called beforehand.
    ///
    /// [`with_ddragon`]: #method.with_ddragon
    pub fn ddragon(&mut self) -> &mut DDragonClient {
        match self.ddragon {
            Some(ref mut dd) => dd,
            None => panic!(
                "You are trying to access ddragon client without creating an embedded one in LeagueClient!\n\
                See with_ddragon method in League Client.\
                ")
        }
    }

    ///Get summoner by plaintext name
    /// # Example
    /// ```
    /// #![feature(async_await)]
    /// #[macro_use]
    /// extern crate tokio;
    /// extern crate narwhalol;
    /// use narwhalol::{LeagueClient, Region, dto::api::Summoner, error::ClientError};
    /// use tokio::prelude::*;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), ClientError> {
    ///     let mut lapi = LeagueClient::new(Region::RU).unwrap();
    ///     let summoner = lapi.get_summoner_by_name("Vetro").await?;
    ///     assert_eq!(summoner.name, "Vetro");
    ///     Ok(())
    /// }
    ///
    /// ```
    ///
    pub fn get_summoner_by_name(
        &mut self,
        name: &str,
    ) -> impl Future<Output = Result<Summoner, ClientError>> {
        println!("Getting summoner with name: {}", &name);
        let url: Uri = format!("{}/summoner/v4/summoners/by-name/{}", self.base_url, name)
            .parse()
            .unwrap();
        debug!("Constructed url: {:?}", &url);
        cached_resp(
            self.client.clone(),
            self.cache.clone(),
            url,
            Some(&self.api_key),
        )
    }

    pub fn get_champion_info(&mut self) -> impl Future<Output = Result<ChampionInfo, ClientError>> {
        let url: Uri = format!("{}/platform/v3/champion-rotations", self.base_url)
            .parse()
            .unwrap();
        cached_resp(
            self.client.clone(),
            self.cache.clone(),
            url,
            Some(&self.api_key),
        )
    }

    pub fn get_champion_masteries(
        &mut self,
        summoner_id: &str,
    ) -> impl Future<Output = Result<Vec<ChampionMastery>, ClientError>> {
        trace!("Getting champion masteries for id: {}", &summoner_id);
        let url: Uri = format!(
            "{}/champion-mastery/v4/champion-masteries/by-summoner/{}",
            self.base_url, summoner_id
        )
        .parse()
        .unwrap();
        cached_resp(
            self.client.clone(),
            self.cache.clone(),
            url,
            Some(&self.api_key),
        )
    }

    pub fn get_champion_mastery_by_id(
        &mut self,
        summoner_id: &str,
        champion_id: u64,
    ) -> impl Future<Output = Result<ChampionMastery, ClientError>> {
        let url: Uri = format!(
            "{}/champion-mastery/v4/champion-masteries/by-summoner/{}/by-champion/{}",
            self.base_url, summoner_id, champion_id
        )
        .parse()
        .unwrap();
        cached_resp(
            self.client.clone(),
            self.cache.clone(),
            url,
            Some(&self.api_key),
        )
    }

    pub fn get_total_mastery_score(
        &mut self,
        summoner_id: &str,
    ) -> impl Future<Output = Result<i32, ClientError>> {
        let url: Uri = format!(
            "{}/champion-mastery/v4/scores/by-summoner/{}",
            self.base_url, summoner_id
        )
        .parse()
        .unwrap();
        cached_resp(
            self.client.clone(),
            self.cache.clone(),
            url,
            Some(&self.api_key),
        )
    }

    pub fn get_league_exp_entries(
        &mut self,
        queue: RankedQueue,
        tier: RankedTier,
        division: Division,
        pages: Option<i32>,
    ) -> impl Future<Output = Result<Vec<LeagueInfo>, ClientError>> {
        let url: Uri = match pages {
            Some(p) => format!(
                "{}/league-exp/v4/entries/{}/{}/{}?page={}",
                &self.base_url, queue, tier, division, p
            )
            .parse()
            .unwrap(),
            None => format!(
                "{}/league-exp/v4/entries/{}/{}/{}",
                &self.base_url, queue, tier, division
            )
            .parse()
            .unwrap(),
        };

        cached_resp(
            self.client.clone(),
            self.cache.clone(),
            url,
            Some(&self.api_key),
        )
    }

    #[cfg(test)]
    pub(crate) fn get_status(&self, status: u16) -> impl Future<Output = Result<(), ClientError>> {
        ClientError::check_status(self.region.clone(), status)
    }
}

impl Default for LeagueClient {
    fn default() -> LeagueClient {
        LeagueClient::new(Region::default()).expect("Please provide API_KEY environment variable")
    }
}

fn check_token(token: &str) -> Result<(), ClientError> {
    if !token.contains("RGAPI") || token.len() != 42_usize {
        WrongToken {
            token: token.to_owned(),
        }
        .fail()
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::LeagueClient;
    use crate::constants::{LanguageCode, RankedQueue, RankedTier, Region};

    use env_logger;
    use futures::prelude::*;
    use futures::{Future, FutureExt, TryFutureExt};

    use crate::constants::division::Division;
    use crate::dto::api::{ChampionInfo, ChampionMastery, Summoner};
    use crate::dto::ddragon::ChampionFullData;
    use crate::error::ClientError;
    use crate::types::Cache;
    use log::debug;
    use std::time::Instant;

    #[cfg(test)]
    fn print_cache(cache: Cache) {
        debug!("{:?}", cache.lock().unwrap().keys().collect::<Vec<_>>())
    }

    #[test]
    fn gets_summoner_data() {
        let mut runtime = tokio::runtime::current_thread::Runtime::new().unwrap();
        env_logger::init();
        let sum: Summoner = runtime
            .block_on(
                LeagueClient::new(Region::NA)
                    .unwrap()
                    .get_summoner_by_name("Santorin"),
            )
            .unwrap();
        assert_eq!(
            &sum.account_id,
            "rPnj4h5W6OhejxB-AO3hLOQctgZcckqV_82N_8_WuCFdO2A"
        )
    }

    #[test]
    fn lapi_caches_properly() {
        let mut runtime = tokio::runtime::current_thread::Runtime::new().unwrap();
        let mut cli = LeagueClient::new(Region::RU).unwrap();
        let cache = cli.cache.clone();
        let _ = runtime.block_on(cli.get_summoner_by_name("Vetro")).unwrap();
        let now = Instant::now();
        let _ = runtime.block_on(cli.get_summoner_by_name("Vetro")).unwrap();
        assert!(now.elapsed().as_millis() <= 2);
        print_cache(cache);
    }

    #[test]
    fn gets_champion_info() {
        let mut lapi = LeagueClient::new(Region::default()).unwrap();
        let mut runtime = tokio::runtime::current_thread::Runtime::new().unwrap();
        let champ_info = runtime.block_on(lapi.get_champion_info()).unwrap();
        assert!(champ_info.free_champion_ids.len() > 10);
        assert!(champ_info.free_champion_ids_for_new_players.len() > 0);
        assert_ne!(champ_info.max_new_player_level, 0)
    }

    #[test]
    fn gets_champion_masteries() {
        let mut lapi = LeagueClient::new(Region::NA).unwrap();
        let mut runtime = tokio::runtime::current_thread::Runtime::new().unwrap();
        let masteries: Vec<ChampionMastery> = runtime
            .block_on(
                lapi.get_summoner_by_name("Santorin")
                    .and_then(move |summoner| lapi.get_champion_masteries(&summoner.id)),
            )
            .unwrap();
        assert_ne!(masteries.len(), 0)
    }

    #[test]
    fn gets_champion_mastery_by_id() {
        let mut runtime = tokio::runtime::current_thread::Runtime::new().unwrap();
        let mut lapi = LeagueClient::new(Region::default())
            .unwrap()
            .with_ddragon(LanguageCode::UNITED_STATES);
        let mut ddragon_client = lapi.ddragon();
        let lee_sin: ChampionFullData = runtime
            .block_on(ddragon_client.get_champion("LeeSin"))
            .unwrap();
        let summoner: Summoner = runtime
            .block_on(lapi.get_summoner_by_name("Santorin"))
            .unwrap();
        let mastery: ChampionMastery = runtime
            .block_on(lapi.get_champion_mastery_by_id(&summoner.id, lee_sin.key.parse().unwrap()))
            .unwrap();

        assert_eq!(mastery.champion_id, 64);
        assert_eq!(mastery.champion_level, 7);
        assert!(mastery.champion_points >= 93748)
    }

    #[test]
    fn gets_total_mastery_score() {
        let mut runtime = tokio::runtime::current_thread::Runtime::new().unwrap();
        let mut lapi = LeagueClient::new(Region::default()).unwrap();
        let summoner: Summoner = runtime
            .block_on(lapi.get_summoner_by_name("Santorin"))
            .unwrap();
        let score = runtime
            .block_on(lapi.get_total_mastery_score(&summoner.id))
            .unwrap();
        assert!(score >= 192)
    }

    #[test]
    fn gets_league_exp() -> Result<(), ClientError> {
        let mut runtime = tokio::runtime::current_thread::Runtime::new().unwrap();
        let mut lapi = LeagueClient::new(Region::default()).unwrap();
        let challengers = runtime
            .block_on(lapi.get_league_exp_entries(
                RankedQueue::SOLO,
                RankedTier::CHALLENGER,
                Division::I,
                None,
            ))
            .unwrap();
        assert!(challengers.len() > 0);
        Ok(())
    }
}
