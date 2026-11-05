# @changesets/cli

## 2.29.7

### Patch Changes

- Updated dependencies [[`957f24e`](https://github.com/changesets/changesets/commit/957f24ed0446494c5709189ae57583f72c716d43)]:
  - @changesets/apply-release-plan@7.0.13

## 2.29.6

### Patch Changes

- [#1712](https://github.com/changesets/changesets/pull/1712) [`a3563b0`](https://github.com/changesets/changesets/commit/a3563b0d6c84e7142541493dea0c89acdc339b93) Thanks [@benmccann](https://github.com/benmccann)! - Switch to maintained fork of `external-editor`

## 2.29.5

### Patch Changes

- [#1693](https://github.com/changesets/changesets/pull/1693) [`6352819`](https://github.com/changesets/changesets/commit/6352819685369daecf31f72e948d0fe92ff33485) Thanks [@Andarist](https://github.com/Andarist)! - Fixed an issue with `workspace:^` and `workspace:~` dependency ranges not being semantically treated as, respectively, `^CURRENT_VERSION` and `~CURRENT_VERSION`. This led to dependent packages being, at times, bumped too often when their dependencies with those ranges were bumped.

- Updated dependencies [[`6352819`](https://github.com/changesets/changesets/commit/6352819685369daecf31f72e948d0fe92ff33485)]:
  - @changesets/assemble-release-plan@6.0.9
  - @changesets/get-release-plan@4.0.13

