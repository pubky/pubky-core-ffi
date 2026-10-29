# Releasing native bindings

## 1. Prepare and validate the version

Update the Cargo package version, regenerate the tracked interface files, and
run the relevant tests.

## 2. Build the iOS release locally

On the release Mac, run:

```bash
./build_ios.sh
./scripts/package_ios_bindings.sh
./scripts/update_swift_package.sh
./scripts/verify_swift_package.sh
```

This creates the ignored release files:

```text
dist/PubkyCore.xcframework.zip
dist/pubky-core-ffi-ios.zip
```

`update_swift_package.sh` records the version and checksum of that exact
`PubkyCore.xcframework.zip` in `Package.swift`. Do not rebuild or repackage the
archive after committing its checksum. XCFramework builds are not reproducible
across machines, including CI runners.

Commit the version, generated interface files, and `Package.swift`, then create
and push the signed version tag.

## 3. Publish the exact iOS archives

Create the GitHub Release from the signed tag and attach the two files produced
in step 2. The SwiftPM asset must be named exactly
`PubkyCore.xcframework.zip`.

For example:

```bash
gh release create "v<VERSION>" \
  dist/PubkyCore.xcframework.zip \
  dist/pubky-core-ffi-ios.zip \
  --generate-notes \
  --verify-tag
```

CI deliberately does not rebuild or publish the iOS release asset. Its iOS
archive is uploaded as `pubky-core-ffi-ios-ci-validation` only for build
validation and will generally have a different checksum.

## 4. Publish Android

Pushing the signed version tag runs the native-bindings workflow. It rebuilds
and verifies the Android AAR, including 16 KB page alignment, then publishes it
to GitHub Packages with the workflow's `GITHUB_TOKEN`.

## 5. Publish Python wheels

The `Python wheels` workflow builds platform-specific wheels from the same tag
on Linux x86_64/arm64, macOS x86_64/arm64, and Windows x86_64. It installs each
wheel and exercises native functions and typed records on Python 3.9 and 3.14.
The Python package version is read from `Cargo.toml`, rather than a generated setup.py.

Before the first PyPI release, a maintainer must:

1. Establish ownership of the `pubkycore` project on PyPI.
2. Register GitHub Trusted Publishing for owner `pubky`, repository
   `pubky-core-ffi`, workflow `python-wheels.yml`, environment `pypi`.
3. Configure required reviewers for the repository's `pypi` environment.
4. Set the repository variable `PYPI_PUBLISH_ENABLED` to `true`.

Until this setup is complete, tag builds produce tested, downloadable wheel
artifacts and skip PyPI publication. Do not advertise the PyPI installation
command as available until the first release is published. A signed tag and
approval of the protected environment publish the exact wheels already tested
by CI; the publish job does not rebuild them. No PyPI token is stored in GitHub.
