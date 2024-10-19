// This file was generated by [ts-rs](https://github.com/Aleph-Alpha/ts-rs). Do not edit this file manually.

/**
 * Specification of a rewarding interval.
 */
export type Interval = {
  /**
   * Monotonously increasing id of this interval.
   */
  id: number;
  /**
   * Number of epochs in this interval.
   */
  epochs_in_interval: number;
  /**
   * The timestamp indicating the start of the current rewarding epoch.
   */
  current_epoch_start: string;
  /**
   * Monotonously increasing id of the current epoch in this interval.
   */
  current_epoch_id: number;
  /**
   * The duration of all epochs in this interval.
   */
  epoch_length: { secs: number; nanos: number };
  /**
   * The total amount of elapsed epochs since the first epoch of the first interval.
   */
  total_elapsed_epochs: number;
};
