use crate::constants::LanguageCode;
use crate::dto::ddragon::{AllChampions, ChampionExtended, ChampionFullData};
use reqwest::{Client, Url};
use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::fmt::Debug;

#[derive(Debug)]
pub struct DDragonClient {
    version: String,
    client: Client,
    base_url: String,
    cache: HashMap<Url, String>,
}

impl DDragonClient {
    pub fn new(language: LanguageCode) -> Result<DDragonClient, reqwest::Error> {
        let client = Client::new();
        let mut versions: Vec<String> = client
            .get("https://ddragon.leagueoflegends.com/api/versions.json")
            .send()?
            .json()?;
        let version = versions.remove(0);
        drop(versions);
        let base_url = format!(
            "http://ddragon.leagueoflegends.com/cdn/{}/data/{}",
            &version, &language
        );
        let cache = HashMap::new();
        let ddragon = DDragonClient {
            version,
            client,
            base_url,
            cache,
        };
        Ok(ddragon)
    }

    pub fn get_champions(&mut self) -> Result<AllChampions, reqwest::Error> {
        let url: Url = format!("{}/champion.json", &self.base_url).parse().unwrap();
        self.get_deserialized_or_add_raw::<AllChampions>(url)
    }

    pub fn get_champion(&mut self, name: &str) -> Result<ChampionFullData, reqwest::Error> {
        let url: Url = format!("{}/champion/{}.json", &self.base_url, name)
            .parse()
            .unwrap();
        let mut ext = self
            .get_deserialized_or_add_raw::<ChampionExtended>(url)
            .unwrap();
        let champ = ext.data.remove(name).unwrap();
        Ok(champ)
    }

    fn get_deserialized_or_add_raw<T>(&mut self, url: Url) -> Result<T, reqwest::Error>
    where
        T: Debug + DeserializeOwned,
    {
        match self.cache.get(&url) {
            Some(resp) => {
                let returnee: T = serde_json::from_str(resp).unwrap();
                Ok(returnee)
            }
            None => {
                let response: String = self.client.get(url.clone()).send()?.text()?;
                self.cache.insert(url.clone(), response);
                let returnee =
                    serde_json::from_str(self.cache.get(&url).unwrap()).expect("Could not parse");
                Ok(returnee)
            }
        }
    }

    #[cfg(test)]
    pub(crate) fn print_cache(&self) {
        println!("cache: {:#?}", self.cache.keys().collect::<Vec<_>>())
    }
}

#[cfg(test)]
mod tests {
    use crate::dto::ddragon::{AllChampions, ChampionFullData};
    use crate::DDRAGON_CLIENT;
    use std::time::Instant;

    #[test]
    fn caches_properly() {
        let mut client = DDRAGON_CLIENT.lock().unwrap();
        let champs = client.get_champions().unwrap();
        drop(champs);
        let now = Instant::now();
        let champs: AllChampions = client.get_champions().unwrap();
        assert!(now.elapsed().as_millis() < 100);
        assert_eq!("103", &champs.data.get("Ahri").unwrap().key);
    }

    #[test]
    fn gets_full_champion_data() {
        let mut client = DDRAGON_CLIENT.lock().unwrap();
        let xayah: ChampionFullData = client.get_champion("Xayah").unwrap();
        assert_eq!(xayah.name, "Xayah");
        client.print_cache()
    }
}
