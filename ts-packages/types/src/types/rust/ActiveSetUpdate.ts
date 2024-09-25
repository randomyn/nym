// This file was generated by [ts-rs](https://github.com/Aleph-Alpha/ts-rs). Do not edit this file manually.

/**
 * Specification on how the active set should be updated.
 */
export type ActiveSetUpdate = {
  /**
   * The expected number of nodes assigned entry gateway role (i.e. [`Role::EntryGateway`])
   */
  entry_gateways: number;
  /**
   * The expected number of nodes assigned exit gateway role (i.e. [`Role::ExitGateway`])
   */
  exit_gateways: number;
  /**
   * The expected number of nodes assigned the 'mixnode' role, i.e. total of [`Role::Layer1`], [`Role::Layer2`] and [`Role::Layer3`].
   */
  mixnodes: number;
};
