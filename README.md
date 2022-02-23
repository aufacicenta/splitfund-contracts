# NEAR Holdings Contracts

NEAR Holdings is a Fractionalized Asset Trading dApp bringing the power of group investing to the masses.

Buy & trade ownership from 1 Ⓝ.

## Resources

- [Working demo](https://near.holdings/)
- [NEAR contracts codebase](https://github.com/aufacicenta/near.holdings)
- [User Interface codebase](https://github.com/aufacicenta/near.holdings-web)

Testnet contracts:

- [escrowfactory.nearholdings.testnet](https://explorer.testnet.near.org/accounts/escrowfactory.nearholdings.testnet)
- [daofactory.nearholdings.testnet](https://explorer.testnet.near.org/accounts/daofactory2.nearholdings.testnet)
- [ftfactory.nearholdings.testnet](https://explorer.testnet.near.org/accounts/ftfactory2.nearholdings.testnet)
- [stakingfactory.nearholdings.testnet](https://explorer.testnet.near.org/accounts/stakingfactory.nearholdings.testnet)

## Motivation (The Problem)

As young adults living in Latin America wanting to invest in their future, we realized that prices to afford an entire land, a house, or investing in a business require either many years of saving or opt for a mortgage.

What if we can invest in these things together, we wondered.

## The Solution

NEAR Holdings is a Fractionalized Asset Trading dApp bringing the power of group investing to the masses. Users can buy & trade ownership from 1 Ⓝ.

### How it works

1. Enter https://near.holdings and publish an asset by setting a title, a price and other details. The asset information is uploaded to IPFS and pinned to the Crust network.
2. Share the asset page for people to invest in with NEAR Tokens
3. Funds can be withdrawn if the price is not reached within the funding period
4. Funds are transferred to a new DAO where the investors become its members if the price is reached within the funding period
5. Investors can claim their share of NEP141 Fungible Tokens

## Use Cases

NEAR Holdings understands the power of Community Capital. NEP141 Fungible Tokens are tied to the value of an asset and a DAO decides on the future of the asset.

NEAR Holdings serves as a launching pod for ideas. Submit an investment idea and measure interest of investors all around the world.

### Art

Invest in an art piece, even in one that doesn't exist yet. Build a new museum or invest in a collection. Make a new movie, or revolutionize the film industry.

### Real Estate

Buy and manage land. Have a stake at the next residential building revolution, earn from the next harvest.

### Events

Pre-purchase tickets and bring your favorite artist to town.

### Commodity (Stock)

Buy a lot of coffee beans, 10 tons of corn, 1,000,000 worth of, socks? Decide on how profit from it.

### Business

Fund a startup, let the community decide on its future.

## Backend Architecture

NEAR Holdings is built from scratch since the beginning of this hackaton.

### Conditional Escrow

[conditional-escrow/src/lib.rs](https://github.com/aufacicenta/near.holdings/blob/master/rust-escrow/conditional-escrow/src/lib.rs)

Responsible for securely holding the funds during the funding period. Funds can be withdrawn if the asset price is not met within the expiration date.

Funds are transferred to a new DAO if the price is met within the funding period.

A new NEP141 token is minted and can be proportionally claimed by the depositors.

```rust
#[near_bindgen]
impl ConditionalEscrow {
    #[init]
    pub fn new(
        expires_at: u64,
        funding_amount_limit: U128,
        dao_factory_account_id: AccountId,
        ft_factory_account_id: AccountId,
        metadata_url: String,
    ) -> Self {
        assert!(!env::state_exists(), "The contract is already initialized");
        Self {
            deposits: UnorderedMap::new(b"r".to_vec()),
            total_funds: 0,
            funding_amount_limit: funding_amount_limit.0,
            unpaid_funding_amount: funding_amount_limit.0,
            expires_at,
            dao_factory_account_id,
            ft_factory_account_id,
            metadata_url,
            dao_name: "".to_string(),
            is_dao_created: false,
        }
    }

   #[payable]
    pub fn deposit(&mut self) {}

   #[payable]
    pub fn withdraw(&mut self) {}

   #[payable]
    pub fn delegate_funds(&mut self, dao_name: String) -> Promise {}
```

### Escrow Factory

[escrow-factory/src/lib.rs](https://github.com/aufacicenta/near.holdings/blob/master/rust-escrow/src/lib.rs)

Responsible for creating `Conditional Escrow` contracts. It keeps a record of all the contracts and has getters for pagination.

```rust
#[near_bindgen]
impl EscrowFactory {
    #[init]
    pub fn new() -> Self {
        assert!(!env::state_exists(), "The contract is already initialized");
        Self {
            conditional_escrow_contracts: UnorderedSet::new(b"d".to_vec()),
        }
    }

     #[payable]
    pub fn create_conditional_escrow(&mut self, name: AccountId, args: Base64VecU8) -> Promise {}
```

### DAO Factory

[dao-factory/src/lib.rs](https://github.com/aufacicenta/near.holdings/blob/master/rust-escrow/dao-factory/src/lib.rs)

Responsible for creating a new Sputnik2 DAO if the asset is funded within the funding period.

The depositors become members of the DAO with voting rights over the proposals.

```rust
#[near_bindgen]
impl DaoFactory {
    #[init]
    pub fn new(dao_factory_account: AccountId) -> Self {
        assert!(!env::state_exists(), "The contract is already initialized");
        Self {
            dao_index: UnorderedMap::new(b"r".to_vec()),
            dao_factory_account,
        }
    }

    #[payable]
    pub fn create_dao(&mut self, dao_name: String, deposits: Vec<String>) -> Promise {}
```

### Fungible Token Factory

[ft-factory/src/lib.rs](https://github.com/aufacicenta/near.holdings/blob/master/rust-escrow/ft-factory/src/lib.rs)

Responsible for minting a new NEP141 Fungible Token proportionally claimable by the `Conditional Escrow` depositors.

```rust
#[near_bindgen]
impl FtFactory {
    #[init]
    pub fn new() -> Self {
        assert!(!env::state_exists(), "ERR_CONTRACT_ALREADY_INITIALIZED");
        Self {
            ft_index: UnorderedMap::new(b"r".to_vec()),
        }
    }

    #[payable]
    pub fn create_ft(&mut self, name: String) -> Promise {}
```

### NEP141 Fungible Token

Meets the NEP141 with an additional `claim` method:

```rust
#[near_bindgen]
impl Ft {
    /// Initializes the contract
    #[init]
    pub fn new(
        max_supply: U128,
        escrow_account_id: AccountId,
        metadata: FungibleTokenMetadata,
    ) -> Self {
        assert!(!env::state_exists(), "ERR_ALREADY_INITIALIZED");
        metadata.assert_valid();
        Self {
            max_supply,
            escrow_account_id,
            token: FungibleToken::new(b"a".to_vec()),
            metadata: LazyOption::new(b"m".to_vec(), Some(&metadata)),
        }
    }

    pub fn claim(&mut self) -> Promise {}
```

### Staking Factory

[staking-factory/src/lib.rs](https://github.com/aufacicenta/near.holdings/blob/master/rust-escrow/staking-factory/src/lib.rs)

[enable_staking](https://github.com/aufacicenta/near.holdings/blob/b32fe7bf2661ef68ef31f3e392c9c6e9d1bc8537/rust-escrow/conditional-escrow/src/lib.rs#L271)

Responsible for making the existing DAO members voting rights proportional to their NEP141 token holdings.

```rust
#[near_bindgen]
impl StakingFactory {
    #[init]
    pub fn new() -> Self {
        assert!(!env::state_exists(), "ERR_CONTRACT_ALREADY_INITIALIZED");
        Self {
            staking_index: UnorderedMap::new(b"r".to_vec()),
        }
    }

   #[payable]
    pub fn create_stake(
        &mut self,
        name: String,
        dao_account_id: AccountId,
        token_account_id: AccountId,
        unstake_period: U64,
    ) -> Promise {}
```

## Client Architecture

[https://github.com/aufacicenta/near.holdings-web](https://github.com/aufacicenta/near.holdings-web/)

NEAR Holdings consists of a NextJS frontend that follows best practices overall.

Its main pages are:

- Homepage [near.holdings](https://near.holdings) — Introduction, FAQs and Featured Active Holdings
- Asset Preview [near.holdings/p/preview](https://near.holdings/p/preview?responseId=k95zme9rxsppqvjfw2gyvjjrk95zme9r) — See how the asset looks like before submitting to the NEAR blockchain
- Asset Explorer [near.holdings/p/explorer](https://near.holdings/p/explorer) — Explore the assets stored in the Escrow Factory contract
- Investment Details [near.holdings/p?contractAddress=](https://near.holdings/p?contractAddress=ce_k95zme9rxsppqvjfw2gyvj.escrowfactory12.nearholdings.testnet) — Deposit, withdraw and delegate the funds of a given Conditional Escrow contract
- Asset Submission [near.holdings](https://near.holdings) — click on "Submit Asset" to enter the submission experience

## Roadmap

March 2022 — NEAR Mainnet launch. We'd like to invest part of the price funds to get a full audit of NEAR Holdings smart contract infrastructure. Some assets will hold thousands, or millions of dollars in NEAR tokens and we want to protect the users of possible vulnerabilities.

March 2022 — Reduce the cost of submitting an asset. EDIT in the demo video it costs 20 NEAR, but we reduced the cost to ~2 NEAR.

March 2022 — Allow to post more media files into an asset metadata, such as video, GIF or PDF files.

April 2022 — Optimized asset explorer. Create an index of all the assets and create an asset explorer page with filters by price, expiration date, categories and even search by description and owners.

April 2022 — Asset subcategories. eg. publish assets under Art: Painting, Art: Sculpture, Art: Digital Illustration, Art: Pottery, etc.

May 2022 — Asset Tools. Once an asset is fully funded and the money lies within the DAO, NEAR Holdings aims to provide tools to make the most out of the asset and the DAO. Convert it to an NFT, provide asset management services, trade your NEP141 tokens in a DEX, among other ideas.

May 2022 — NEAR Holdings go full thrust all around the world. Once it is fully tested in mainnet and costs are reduced to a minimum, we'll launch a worldwide campaign with hand-picked assets to invest in. **We are talking about Real Estate, real-world art pieces, commodities (stock), events and businesses that are financed within minutes and developed & maintained by DAOs!**


## How does NEAR Holdings contribute to Web3, a new internet?

Traditional investment trusts and other investors' societies have always been limited to the wealthy. NEAR Holdings opens the investment world to anyone with a NEAR wallet allowing people not only to hold a collateral over the real value of an asset, but to be part of the proposals and the future of it.

## What types of projects does NEAR Holdings wants to see financed & developed?

Everything that improves the quality of life. For instance, we provide 5 categories: Art, Real Estate, Commodities (Stock), Businesses and Events.

Can investment opportunities published in NEAR Holdings finance the development of new hospitals in underdeveloped countries? A new space travel company? A Netflix competition? A new music festival? A new farm?

## Crust Network

[NEAR Holdings is applicable for the Crust Network sponsor price too](https://github.com/aufacicenta/near.holdings-web/blob/master/app/src/providers/typeform/getResponseById.ts#L84)

## Building and deploying your own

Simply enter the `rust-escrow` directoy and run: `sh build.sh`

Make sure you already have Rust and Cargo installed.

Once the build is complete, you should be able to deploy your own contracts:

```
near deploy --wasmFile target/wasm32-unknown-unknown/release/escrow_factory.wasm --accountId escrowfactory.nearholdings.testnet --initFunction new --initArgs '{}'
```

```
near deploy --wasmFile target/wasm32-unknown-unknown/release/ft_factory.wasm --accountId ftfactory2.nearholdings.testnet
```

```
near deploy --wasmFile target/wasm32-unknown-unknown/release/dao_factory.wasm --accountId daofactory2.nearholdings.testnet --initFunction new --initArgs '{"dao_factory_account":"sputnikv2.testnet"}'
```