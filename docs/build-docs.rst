Build documentation
-------------------

Documentation build by `sphinx`_ and hosted on `readthedocs`_ service.

.. seealso::

  * `Good tutorials <http://rest-sphinx-memo.readthedocs.io/en/latest/>`__

Local
=====

To build documentation on local machine use virtualenv:

.. code-block:: shell

  > cd docs
  [docs]> virtualenv _venv
  [docs]> source _venv/bin/activate
  (_venv) [docs]> which pip
  /.../docs/_venv/bin/pip

Install dependencies:

.. code-block:: shell

  (_venv) [docs]> pip install -U pip
  (_venv) [docs]> pip install -r requirements.txt

Build documentation:

.. code-block:: shell

  (_venv) [docs]> mkdir _static # if not exists
  (_venv) [docs]> sphinx-build -v -W . _build

Open ``index.html`` in browser:

.. code-block:: shell

  (_venv) [docs]> ls _build/index.html
  _build/index.html

Run spell checker:

.. code-block:: shell

  (_venv) [docs]> sphinx-build -b spelling . _spelling

Helper scripts
==============

Alternatively you can use ``jenkins.sh`` and ``make.sh`` scripts.

To initialize environment, build documentation and run spell checker at once:

.. code-block:: shell

  > cd docs
  [docs]> ./jenkins.sh

Do the same and stay in environment:

.. code-block:: shell

  > cd docs
  [docs]> source ./jenkins.sh
  (_venv) [docs]> which pip
  /.../docs/_venv/bin/pip

``jenkins.sh`` will build documentation from scratch. This may be useful in
case some HTML/CSS files stuck in temporary directory. To update documentation
without deleting old files use ``make.sh`` script (which is usually much faster):

.. code-block:: shell

  (_venv) [docs]> ./make.sh
  ...
  Done:
  /.../docs/_build/index.html

.. _sphinx: http://www.sphinx-doc.org/en/stable/
.. _readthedocs: https://readthedocs.org/
