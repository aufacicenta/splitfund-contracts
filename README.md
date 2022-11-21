# Splitfund Contracts

**Sp/itfund is a co-ownership protocol built on NEAR.**

For the end user, Sp/itfund looks and feels like a Real Estate crowdfunding site: funding amount goal, days left, number of backers, etc. Sp/itfund makes it easy to invest in Real Estate co-ownership starting from 50.00 USDT/NEAR.

On the technical side, users get back a new NEP141 token as collateral for their USDT/NEAR investment. This collateral is tradable at any time. If the funding amount goal is not reached within the expiration date, the users get their investment back.

- Sp/itfund is decentralized, all of the information about the property is stored in the blockchain.
- Sp/itfund charges a 2% fee upon each USDT/NEAR investment, this fee is non-refundable and is used to keep the platform running.
- Sp/itfund manages the Real Estate assets to guarantee the properties' maintenance and profitability. Users can trust Sp/itfund as their investment manager with solid legal representatives in each jurisdiction.
- Sp/itfund makes it easy to make cross-country Real Estate investments. A user in Japan may co-own Real Estate in Mexico, for example.
- Sp/itfund opens a new market of tradable NEP141 Real Estate collateral tokens as a **store of value**.
- Once Sp/itfund buys a Real Estate property, our legal mechanism will make each investor's wallet a legal owner of the asset.

## Current features

- [x] `deposit` send an amount of a given NEP141 token via `ft_transfer_call` and get a new NEP141 in exchange as a receipt 1:1 for your investment
- [x] `withdraw` if the total funding amount is not reached within the expiration date, get your NEP141 deposit back
- [x] `delegate_funds` if the total funding amount is reached, send the NEP141 balance to a given wallet
- [x] Unit tests complete
- [x] To deploy an instance of the Escrow contract, either use `near deploy` or use [the factory](https://github.com/aufacicenta/splitfund-contracts/blob/master/factory/src/lib.rs)

## Documentation

See the [examples](https://github.com/aufacicenta/splitfund-contracts/tree/master/escrow-v2).

### Using near-api-js

Refer to [this file](https://github.com/aufacicenta/splitfund/blob/master/app/src/pages/api/webhooks/splitfund/strapi-entry-update/index.ts#L86).