/* generated using openapi-typescript-codegen -- do no edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { InputEndpointConfig } from './InputEndpointConfig'
import type { OutputEndpointConfig } from './OutputEndpointConfig'
import type { ResourceConfig } from './ResourceConfig'

/**
 * Pipeline configuration specified by the user when creating
 * a new pipeline instance.
 *
 * This is the shape of the overall pipeline configuration, but is not
 * the publicly exposed type with which users configure pipelines.
 */
export type PipelineConfig = {
  /**
   * Enable CPU profiler.
   */
  cpu_profiler?: boolean
  /**
   * Maximal delay in microseconds to wait for `min_batch_size_records` to
   * get buffered by the controller, defaults to 0.
   */
  max_buffering_delay_usecs?: number
  /**
   * Minimal input batch size.
   *
   * The controller delays pushing input records to the circuit until at
   * least `min_batch_size_records` records have been received (total
   * across all endpoints) or `max_buffering_delay_usecs` microseconds
   * have passed since at least one input records has been buffered.
   * Defaults to 0.
   */
  min_batch_size_records?: number
  resources?: ResourceConfig
  /**
   * Number of DBSP worker threads.
   */
  workers?: number
} & {
  /**
   * Input endpoint configuration.
   */
  inputs: Record<string, InputEndpointConfig>
  /**
   * Pipeline name
   */
  name?: string | null
  /**
   * Output endpoint configuration.
   */
  outputs?: Record<string, OutputEndpointConfig>
}
