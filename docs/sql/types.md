# Supported Data Types

The compiler supports the following SQL data types:

- `BOOLEAN`, or `BOOL`, represented as a Boolean value
- `TINYINT`, represented as an 8-bit signed integer using two's
  complement.
- `SMALLINT`, or `INT2`, represented as a 16-bit signed integer using two's
  complement.
- `INTEGER`, or `INT`, or `SIGNED`, or `INT4`, represented as a 32-bit signed integer,
  using two's complement
- `BIGINT`, or `INT64`, represented as a 64-bit signed integer, using two's
  complement
- `DECIMAL(`precision`,`scale`)`, or `DEC(`precision`,` scale`)`, or
  `NUMERIC(`precision`,` scale`)`, or `NUMBER(`precision`,` scale`)`, a high
  precision fixed-point type, with a precision (total number of decimal
  digits) and a scale (number of decimal digits after period).  For example,
  23.456 has a precision of 5 and a scale of 3.  If scale is missing it is
  assumed to be 0.
- `REAL`, or `FLOAT4`, or `FLOAT32`, an IEEE 32-bit floating point number
- `DOUBLE`, or `FLOAT8`, or `FLOAT32`, an IEEE 64-bit floating point number
- `VARCHAR(n)`, or `CHARACTER VARYING(n)`, a string value with maximum fixed width
  Trailing spaces are removed when converting a value to one of these types.
- `CHAR(n)`, or `CHARACTER(n)`, a string value with a fixed width.  Values
  are truncated if longer, or padded with spaces if shorter, to be brought to
  the specified size.
- `VARCHAR`, or `STRING`, or `TEXT`: a string of unlimited length.  Trailing
   spaces are removed when converting a `CHAR(n)` value to this type.
- `BINARY(n)`, a byte string with a fixed width
- `VARBINARY`, a byte string with an unlimited width.  `BYTEA` is an alias.
- `NULL`, a type comprising only the `NULL` value
- `INTERVAL`, a SQL interval.  Two types of intervals are supported:
  long intervals (comprising years and months), and short intervals,
  comprising days, hours, minutes, seconds.
- `TIME`, time of the day.
- `TIMESTAMP`, a SQL timestamp without a timezone.  A timestamp
  represents a value containing a date and a time.
- `DATE`, a SQL date without a timezone.  A date represents a value
  containing a date (year, month, day).
- `GEOMETRY`: geographic data type (only rudimentary support at this point)
- `ARRAY`: used as a suffix for another type, as in `INT ARRAY`.
  An array with element of the specified type.


A suffix of `NULL` or `NOT NULL` can be appended to a type name to
indicate the nullability.  A type with no suffix is not nullable by
default.

Note: the `FLOAT` type is not supported.  Please use `REAL` or
`DOUBLE` instead.  Various SQL dialects do not agree on the size of
the `FLOAT` type, so we have decided to prohibit its use to avoid
subtle bugs.

## Computations on nullable types

A type is nullable if it can represent the `NULL` value.  For input
tables the nullability of a column is declared explicitly.  For
intermediate results and output views the compiler infers the
nullability of each column using type inference rules.

Most SQL operations are defined for nullable types.  Our compiler
follows the SQL standard in this respect.  Most operations (e.g.,
`+`), when applied a `NULL` operand will produce a `NULL`
value.

## Grammar for specifying types

```
type:
      typeName
      [ collectionsTypeName ]*

typeName:
      sqlTypeName
  |   compoundIdentifier

sqlTypeName:
      char [ precision ] [ charSet ]
  |   varchar [ precision ] [ charSet ]
  |   DATE
  |   time
  |   timestamp
  |   GEOMETRY
  |   decimal [ precision [, scale] ]
  |   BOOLEAN
  |   integer
  |   BINARY [ precision ]
  |   varbinary [ precision ]
  |   TINYINT
  |   SMALLINT
  |   BIGINT
  |   REAL
  |   double

collectionsTypeName:
      ARRAY

char:
      CHARACTER | CHAR

varchar:
      char VARYING | VARCHAR

decimal:
      DECIMAL | DEC | NUMERIC | NUMBER

integer:
      INTEGER | INT

varbinary:
      BINARY VARYING | VARBINARY

double:
      DOUBLE [ PRECISION ]

time:
      TIME [ precision ] [ timeZone ]

timestamp:
      TIMESTAMP [ precision ] [ timeZone ]

charSet:
      CHARACTER SET charSetName

timeZone:
      WITHOUT TIME ZONE
```

A `compoundIdentifier` is a sequence of identifiers separated by dots.
