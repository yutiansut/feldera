/* generated using openapi-typescript-codegen -- do no edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

/**
 * A SQL column type description.
 *
 * Matches the Calcite JSON format.
 */
export type ColumnType = {
  component?: ColumnType | null
  /**
   * Does the type accept NULL values?
   */
  nullable: boolean
  /**
   * Precision of the type.
   *
   * # Examples
   * - `VARCHAR` sets precision to `-1`.
   * - `VARCHAR(255)` sets precision to `255`.
   * - `BIGINT`, `DATE`, `FLOAT`, `DOUBLE`, `GEOMETRY`, etc. sets precision
   * to None
   * - `TIME`, `TIMESTAMP` set precision to `0`.
   */
  precision?: number | null
  /**
   * The scale of the type.
   *
   * # Example
   * - `DECIMAL(1,2)` sets scale to `2`.
   */
  scale?: number | null
  /**
   * Identifier for the type (e.g., `VARCHAR`, `BIGINT`, `ARRAY` etc.)
   */
  type: string
}
