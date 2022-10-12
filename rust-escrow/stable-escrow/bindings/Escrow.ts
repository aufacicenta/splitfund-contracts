// This file was generated by [ts-rs](https://github.com/Aleph-Alpha/ts-rs). Do not edit this file manually.
import type { DAO } from "./DAO";
import type { Fees } from "./Fees";
import type { Metadata } from "./Metadata";
import type { Staking } from "./Staking";

export interface Escrow { deposits: string[], ft: FungibleToken, ft_metadata: FungibleTokenMetadata, metadata: Metadata, dao: DAO, fees: Fees, staking: Staking, }