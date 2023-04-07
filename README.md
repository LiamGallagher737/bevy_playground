## Overview

- `compiler` The http server that accepts api requests for compiling projects

- `compiler/app` On every compile request this container will be created inside of the `compiler` container

## Commands

Build compiler server image

```
cd compiler
docker build -t liamg737/bevy_playground:0.0.1 .
```

Run compiler server image

```
docker run --rm -v "/var/run/docker.sock:/var/run/docker.sock" -p 8080:8080 liamg737/bevy_playground:0.0.1
```

```
docker run --rm --group-add 0 -v "/var/run/docker.sock:/var/run/docker.sock" -p 8080:8080 liamg737/bevy_playground:0.0.1
```
