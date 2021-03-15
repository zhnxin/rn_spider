# rn_spider 

A spider build with rust to store web text selected by css selector.

## Usage

```
rn_spider -c <config.toml> <output file>
```

Rhe ENV https_proxy and http_proxy can be set for proxy.

## config file

```toml
base = 'https://example.com/views?id='
content = 'div.content'
title = 'div.h1.title'
url_list = [
    '1097436230',
    '1036253449'
]
```
