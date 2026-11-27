# Release packages

These packages use the native binaries and SHA-256 checksums from the release
workflow. Publish a GitHub release before advertising the packages.

## npm, Bun, pnpm, and Yarn

Download all six binary archives and their checksums, then build the package:

```sh
gh release download v0.6.1 --dir /tmp/hive-assets
python3 scripts/package-npm.py v0.6.1 /tmp/hive-assets /tmp/hive-packages
npm publish /tmp/hive-packages/codefrontlabs-hive-0.6.1.tgz --access public
```

The package ships a single launcher and both native executables for each
supported platform. It uses no install scripts. Test a fresh install and
`hive --version` before publishing.

## Arch Linux

Update the version and per-architecture release archive checksums in
[`aur/PKGBUILD`](aur/PKGBUILD). Reset `pkgrel` to 1 for a new Hive version.
From that directory, run `makepkg --printsrcinfo > .SRCINFO` and `makepkg` as
a regular user. The build produces a package for `pacman -U`. Publish the
package and its checksum with the GitHub release, and publish the recipe to
the AUR using an authorized maintainer account.

## Homebrew

Update the version, URLs and checksums in
[`Formula/codefront-hive.rb`](../Formula/codefront-hive.rb).
The GitHub repository doubles as the tap. Test a fresh
`brew install jrh89/hive/codefront-hive` and
`brew test jrh89/hive/codefront-hive` before advertising the update.
