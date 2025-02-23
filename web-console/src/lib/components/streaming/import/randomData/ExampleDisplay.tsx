// Displays an example value of a value that was generated by a random
// generation method.
//
// Also indicates if the example was modified after generation due to type
// constraints (overflow etc.).

import { inRangeInclusive } from '$lib/functions/common/bigNumber'
import { ColumnTypeJS, getValueFormatter, numericRange } from '$lib/functions/ddl'
import { Field } from '$lib/services/manager'
import { BigNumber } from 'bignumber.js'

import { Grid, Typography } from '@mui/material'

export const ExampleDisplay = ({
  example,
  field,
  parsed
}: {
  field: Field
  parsed: ColumnTypeJS
  example: ColumnTypeJS
}) => {
  const toDisplay = getValueFormatter(field.columntype)

  let beforeParsedValue = ''
  const displayParsed = toDisplay(parsed)
  let afterParsedValue = ''

  // Indicates if the value got adjusted by the valueparser due to constraints
  // on the field type
  const adjustments: string[] = []
  if (
    typeof example === 'number' &&
    ['TINYINT', 'SMALLINT', 'INTEGER', 'BIGINT', 'DECIMAL', 'FLOAT', 'DOUBLE'].includes(field.columntype.type)
  ) {
    const range = numericRange(field.columntype)
    if (!inRangeInclusive(range)(example)) {
      adjustments.push('clamp')
    }
  }
  if (typeof example === 'string') {
    beforeParsedValue = "'"
    afterParsedValue = "'"
    if (
      ['VARCHAR', 'CHAR'].includes(field.columntype.type) &&
      field.columntype.precision != null &&
      field.columntype.precision != -1 &&
      example.length > field.columntype.precision
    ) {
      adjustments.push('trimmed')
    } else if (
      ['CHAR'].includes(field.columntype.type) &&
      field.columntype.precision != null &&
      example.length < field.columntype.precision
    ) {
      // HTML has this behaviour that if a string has multiple white-space at
      // the end they get reduced to one. Which is a bit odd for displaying the
      // char. As e.g., 'abc' with char(10) is displayed as 'abc<1space>'
      // instead of 'abc<10space>'
      //
      // TODO: fix with ideas from here:
      // https://stackoverflow.com/questions/433493/why-does-html-require-that-multiple-spaces-show-up-as-a-single-space-in-the-brow
      adjustments.push('padded')
    }
  }

  if (
    BigNumber.isBigNumber(example) &&
    typeof parsed === 'string' &&
    ['DECIMAL'].includes(field.columntype.type) &&
    parsed.length != example.toFixed().length
  ) {
    adjustments.push('trimmed')
  }

  return (
    <Grid item sm={2} xs={12}>
      <>
        <Typography sx={{ typography: 'subtitle2' }}>
          Example{adjustments.length > 0 ? ': ' + adjustments.join(',') : ':'}
        </Typography>
        <Typography sx={{ typography: 'body2', fontStyle: 'italic' }}>
          {beforeParsedValue}
          {displayParsed}
          {afterParsedValue}
        </Typography>
      </>
    </Grid>
  )
}
