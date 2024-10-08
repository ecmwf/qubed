
****************************
python-package-template-repo
****************************

|license| |tag_release| |commits_since_release| |last_commit|

A template repository for developing Python packages under `ecmwf-projects <https://github.com/ecmwf-projects>`_.

**Quick start**

Follow these steps to create a new repository from this template.

#. Click the `Use this template <https://github.com/ecmwf-projects/python-package-template-repo/generate>`_
   button and create a new repository with your desired name, location and visibility.

#. Clone the repository::

     git clone git@github.com:ecmwf-projects/<your-repository-name>.git
     cd <your-repository-name>

#. Remove sample code::

     rm PACKAGE_NAME/sample.py
     rm tests/test_sample.py

#. Replace ``PACKAGE_NAME`` with your chosen package name::

     NEW_PACKAGE_NAME=<your-package-name>
     mv PACKAGE_NAME $NEW_PACKAGE_NAME
     sed -i "" "s/PACKAGE_NAME/$NEW_PACKAGE_NAME/g" setup.py \
        docs/source/conf.py \
        docs/source/getting_started/installing.rst \
        docs/source/index.rst \
        $NEW_PACKAGE_NAME/__meta__.py

#. Modify the contents of ``__meta__.py`` to reflect your repository. Note that there
   is no need to update this same information in ``setup.py``, as it will be imported
   directly from ``__meta__.py``.

#. Modify the project url in ``setup.py`` to reflect your project's home in GitHub.

#. Modify ``README.rst`` to reflect your repository. A number of `shield <https://shields.io/>`_
   templates are included, and will need to be updated to match your repository if you want
   to use them.

**Usage tips**

* Create an executable called ``qa`` containing the following::

    black .
    isort .

  Add this to your path, and run it from the top-level of your repository before
  committing changes::

    qa .

.. |last_commit| image:: https://img.shields.io/github/last-commit/ecmwf-projects/thermofeel
    :target: https://github.com/ecmwf-projects/thermofeel

.. |commits_since_release| image:: https://img.shields.io/github/commits-since/ecmwf-projects/thermofeel/latest?sort=semver
    :target: https://github.com/ecmwf-projects/thermofeel

.. |license| image:: https://img.shields.io/github/license/ecmwf-projects/thermofeel
    :target: https://www.apache.org/licenses/LICENSE-2.0.html

.. |pypi_release| image:: https://img.shields.io/pypi/v/thermofeel?color=green
    :target: https://pypi.org/project/thermofeel

.. |pypi_status| image:: https://img.shields.io/pypi/status/thermofeel
    :target: https://pypi.org/project/thermofeel

.. |tag_release| image:: https://img.shields.io/github/v/release/ecmwf-projects/thermofeel?sort=semver
    :target: https://github.com/ecmwf-projects/thermofeel

.. |codecov| image:: https://codecov.io/gh/ecmwf-projects/thermofeel/branch/master/graph/badge.svg
  :target: https://codecov.io/gh/ecmwf-projects/thermofeel

.. |ci| image:: https://img.shields.io/github/workflow/status/ecmwf-projects/thermofeel/ci
  :target: https://github.com/ecmwf-projects/thermofeel/actions

.. |pypi_downloads| image:: https://img.shields.io/pypi/dm/thermofeel
  :target: https://pypi.org/project/thermofeel

.. |code_size| image:: https://img.shields.io/github/languages/code-size/ecmwf-projects/thermofeel?color=green
  :target: https://github.com/ecmwf-projects/thermofeel

.. |docs| image:: https://readthedocs.org/projects/thermofeel/badge/?version=latest
  :target: https://thermofeel.readthedocs.io/en/latest/?badge=latest
