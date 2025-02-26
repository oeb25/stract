# Contributing to Cuely
There are many ways to contribute to Cuely.
Code contribution are welcome of course, but also
bug reports, feature request, and evangelizing are as valuable.

# Submitting a PR
Check if your issue is already listed [github](https://github.com/cuely/cuely/issues).
If it is not, create your own issue.

Please add the following phrase at the end of your commit.  `Closes #<Issue Number>`.
It will automatically link your PR in the issue page. Also, once your PR is merged, it will
closes the issue. If your PR only partially addresses the issue and you would like to
keep it open, just write `See #<Issue Number>`.

Feel free to send your contribution in an unfinished state to get early feedback.
In that case, simply mark the PR with the tag [WIP] (standing for work in progress).

# Signing the CLA
Cuely is an opensource project licensed a AGPLv3.

Contributors are required to sign a Contributor License Agreement.
The process is simple and fast. Upon your first pull request, you will be prompted to
[sign our CLA by visiting this link](https://cla-assistant.io/cuely/cuely).

# Development
## Setup
* Install rust by following the steps outlined [here](https://www.rust-lang.org/tools/install)
    * Install the beta toolchain `rustup toolchain install beta`
    * Set beta as default `rustup default beta`
* Install clang and npm
* Update ulimit. RocksDB tends to exceed the max number of allowed open files, so you will have to run `ulimit -n 10240` to increase the allowed max number of open files.
* Build the frontend `cd frontend && npm install && npm run build`
* (Optional) Install cargo-watch by running `cargo install cargo-watch`. This makes frontend development easier.
* (Optional) Install just by running `cargo install just`. This allows you to run the scripts in the Justfile. A Justfile is basically a simple Makefile, so this is just for convenience.
  
After the non optional steps you can now run `cargo test` and should see all tests passing. If you have followed the optional steps, you should be able to run `just local` which will index some warc files and launch the frontend on `0.0.0.0:3000`.
After you have run the intial indexing, you can run `just frontend` to reuse the index.
