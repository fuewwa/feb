feb (fast & easy build)
=======================

feb is a small build tool in the spirit of GNU Make. Instead of a ``Makefile`` it reads a file called ``febconf``.

Install
-------

feb is written in Rust and built with cargo. Install the Rust toolchain first, then build feb.

Arch Linux
~~~~~~~~~~

.. code-block:: sh

    sudo pacman -S --needed base-devel git rust

Debian / Ubuntu
~~~~~~~~~~~~~~~

.. code-block:: sh

    sudo apt update
    sudo apt install -y build-essential curl git
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
    . "$HOME/.cargo/env"

macOS
~~~~~

.. code-block:: sh

    xcode-select --install
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
    . "$HOME/.cargo/env"

Windows
~~~~~~~

1. Install the Visual Studio Build Tools with the "Desktop development with C++" workload.
2. Install rustup in PowerShell:

.. code-block:: powershell

    winget install Rustlang.Rustup
    rustup default stable

Restart the terminal afterwards so that ``cargo`` is in ``PATH``.

Build and install feb
~~~~~~~~~~~~~~~~~~~~~

.. code-block:: sh

    cargo install --path .
    feb --help

The binary is placed in ``~/.cargo/bin`` (``%USERPROFILE%\.cargo\bin`` on Windows). Make sure that directory is in ``PATH``.

To build without installing:

.. code-block:: sh

    cargo build --release

The binary is then ``target/release/feb``.

febconf
-------

.. code-block:: text

    param = "ProgFile" // comment
    count = 4
    ratio = 1.5
    verbose = true
    other = 'single quotes work too'

    run {
    	python3 ${param}.py
    }

    cppver {
    	mkdir -p build
    	g++ -std=c++17 -Wall -Wextra -O2 -c main.cpp -o build/main.o
    	g++ -o build/main build/main.o
    }

    clear {
    	rm -rf build
    }

In the example above every command line starts with one real TAB character.

Rules:

- ``name = value`` defines a param. The value is a bool (``true`` or ``false``), an int, a double or a string.
- A string must be written in double quotes or in single quotes. A string cannot contain the quote character it is written with.
- ``// text`` starts a comment.
- ``name { ... }`` defines a target.
- Every command line inside a target must start with a TAB character, not spaces.
- A param is inserted into a command with ``${name}``.
- Commands run in order through ``sh -c`` (``cmd /C`` on Windows). The first failing command stops the build.

Conditions
~~~~~~~~~~

.. code-block:: text

    lang = "cpp"

    build {
    	g++ main.cpp -o prog
    	if lang == "py":
    		echo "Python"
    	else
    		echo "C++"
    	fi
    }

- ``if left op right:`` starts a condition. Each side is a param name or a value.
- ``else`` is optional. ``fi`` ends the condition.
- Lines inside a branch start with one more TAB than the ``if`` line. Conditions can be nested.
- ``==`` is equal and ``!=`` is not equal. Both sides must have the same type, except that an int and a double are compared as numbers.
- ``>=`` is greater, ``<=`` is less, ``>!`` is greater or equal and ``<!`` is less or equal. These four work only with numbers. Note that ``>=`` and ``<=`` do not include equality.

Usage
-----

.. code-block:: sh

    feb run
    feb cppver
    feb clear
    feb cppver run
    feb --list

Without arguments feb builds the first target in ``febconf``.

License
-------

GPL-3.0
