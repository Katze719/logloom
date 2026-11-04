# Debian Changelog

Yes, the spaces matter.

```sh
logloom parse examples/debian.changelog --format debian-changelog
```

```text
lantern (1:2.0.0-1) unstable; urgency=medium

  * New upstream release.
    Preserve Unicode paths. Closes: #123456

 -- Alice Example <alice@example.com>  Wed, 04 Nov 2026 12:30:00 +0000
```

| Part | Model |
| --- | --- |
| `1:2.0.0-1` | Opaque version, retaining epoch and Debian revision |
| Trailer date | Normalized ISO `date`; original RFC timestamp in attributes |
| Package, distributions, urgency, other settings | Release attributes |
| Maintainer name/email | `attributes.maintainer` |
| `  [ Contributor ]` | Custom section heading |
| Indented changes | Ordered entries, retaining continuation lines and bug references |

Start the header at column one. Indent changes by at least two spaces, start the
trailer with ` -- `, and put exactly two spaces between maintainer and timestamp.
The header needs a package, version, distributions, and urgency.

Missing trailers, broken settings, and invalid clock/timezone syntax get parsing
errors. Impossible calendar dates get validation errors, pointing at the trailer
through `date_span`.

The original timestamp and numeric timezone stay in attributes. The ISO date uses
the written local date. Weekday/date agreement isn't checked; editor footers and
older alternative layouts aren't interpreted.

Source: [Debian Policy §4.4](https://www.debian.org/doc/debian-policy/ch-source.html#debian-changelog-debian-changelog).
