Installing Rust
===============

.. seealso::

  * `Official way to install Rust <https://doc.rust-lang.org/book/getting-started.html#installing-on-linux-or-mac>`__
  * `Uninstalling Rust <https://doc.rust-lang.org/book/getting-started.html#uninstalling>`__
  * `Downloads <https://www.rust-lang.org/en-US/downloads.html>`__

To install Rust from ``.tar.gz`` archives you need to download ``rust`` and
``rust-std`` archives and `merge them`_:

.. code-block:: shell

  > wget https://static.rust-lang.org/dist/rust-std-1.10.0-x86_64-unknown-linux-gnu.tar.gz
  > tar xf rust-std-1.10.0-x86_64-unknown-linux-gnu.tar.gz

  > wget https://static.rust-lang.org/dist/rust-1.10.0-x86_64-unknown-linux-gnu.tar.gz
  > tar xf rust-1.10.0-x86_64-unknown-linux-gnu.tar.gz

  > mv rust-std-1.10.0-x86_64-unknown-linux-gnu/rust-std-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu rust-1.10.0-x86_64-unknown-linux-gnu/rustc/lib/rustlib

Now add ``rustc`` and ``cargo`` to ``PATH``:

.. code-block:: shell

  > export PATH=/.../rust-1.10.0-x86_64-unknown-linux-gnu/rustc/bin:$PATH
  > export PATH=/.../rust-1.10.0-x86_64-unknown-linux-gnu/cargo/bin:$PATH

Check tools can be found:

.. code-block:: shell

  > which cargo
  /.../rust-1.10.0-x86_64-unknown-linux-gnu/cargo/bin/cargo
  > cargo --version
  cargo 0.11.0-nightly (259324c 2016-05-20)

  > which rustc
  /.../rust-1.10.0-x86_64-unknown-linux-gnu/rustc/bin/rustc
  > rustc --version
  rustc 1.10.0 (cfcb716cf 2016-07-03)

Run build:

.. code-block:: shell

  > cargo build --verbose

.. _merge them: https://users.rust-lang.org/t/cant-find-create-for-std/3464/5
