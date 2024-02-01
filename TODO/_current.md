# Documentation

* Add build dependencies for each operating system on the main `README.md`.
    - Windows:
        - `do prebuild` requires clang
            - needs `CC` and `CXX` environment variables set to `clang-cl`
    - Ubuntu:
```
cargo do install --accept [ok]
cargo do prebuild [ok]
cargo do run icon [error]
sudo apt-get install pkg-config
sudo apt-get install libssl-dev
sudo apt-get install libfontconfig1-dev
cargo do run icon [ok]
```

* Add `description`, `documentation`, `repository`, `readme`, `categories`, `keywords`.
    - Review what other large crates do.
    - Review badges.
* Review docs.
    - Do a full read, look for typos or failed links.

# Publish

* Publish if there is no missing component that could cause a core API refactor.

* Rename crates (replace zero-ui with something that has no hyphen). 
    - Z meaning depth opens some possibilities, unfortunately "zui" is already taken.
    - `znest`: Z-dimension (depth) + nest, Z-Nest, US pronunciation "zee nest"? 
    - `zerui`.
    - `nestui`.

* Review prebuild distribution.
* Pick license and code of conduct.
* Create a GitHub user for the project?
* Create issues for each TODO.

* Publish (after all TODOs in this file resolved).
* Announce in social media.

* After publish only use pull requests.
    - We used a lot of partial commits during development.
    - Is that a problem in git history?
    - Research how other projects handled this issue.