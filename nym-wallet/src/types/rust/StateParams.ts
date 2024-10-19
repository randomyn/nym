// This file was generated by [ts-rs](https://github.com/Aleph-Alpha/ts-rs). Do not edit this file manually.
import type { DecCoin } from '@nymproject/types/src/types/rust/DecCoin';
import type { TauriOperatingCostRange } from './OperatingCostRange';
import type { TauriProfitMarginRange } from './ProfitMarginRange';

export type TauriContractStateParams = {
  minimum_pledge: DecCoin;
  minimum_delegation: DecCoin | null;
  operating_cost: TauriOperatingCostRange;
  profit_margin: TauriProfitMarginRange;
};
