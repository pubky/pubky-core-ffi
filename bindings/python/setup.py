from setuptools import setup, find_packages

setup(
    name="pubkycore",
    version="0.1.0",
    packages=find_packages(),
    package_data={
        "pubkycore": ["*.so", "*.dylib", "*.dll"],
    },
    install_requires=[],
    author="Pubky",
    author_email="",
    description="Python bindings for the Pubky Mobile SDK",
    long_description=open("README.md").read(),
    long_description_content_type="text/markdown",
    url="",
    classifiers=[
        "Programming Language :: Python :: 3",
        "License :: OSI Approved :: MIT License",
        "Operating System :: OS Independent",
    ],
    python_requires=">=3.6",
)
