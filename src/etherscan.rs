use std::fmt;

pub struct Balances<'a> {
  coins: Vec<Balance<'a>>	
}

impl<'a> fmt::Display for Balances<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    	for balance in &self.coins {
        	write!(f, "{},", balance);
        }
        write!(f, "")
    }
}

pub struct Balance<'a> {
  symbol: &'a str,
  amount: f64,
}

impl<'a> fmt::Display for Balance<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}: {}", self.symbol, self.amount)
    }
}

static ETHERSCAN_BASE_API_URL: &'static str = "https://api.etherscan.io/api?";
// "https://api.etherscan.io/api?module=account&action=tokenbalance&contractaddress=0x57d90b64a1a57749b0f932f1a3395792e12e7055&address=0xe04f27eb70e025b78871a2ad7eabe85e61212761&tag=latest&apikey=YourApiKeyToken";

pub fn balances(priv_key_str: &str) -> Balances {
	let coins: Vec<Balance> = Vec::new();
	let b = Balance{symbol: "a", amount: 0.1};
	Balances{coins: vec![b]}
}
