FROM docker.io/library/rust:1.88.0

RUN apt-get update && apt-get install -y \
    build-essential \
    gcc \
    curl \
    git \
    gcc-aarch64-linux-gnu \
    ffmpeg \
    sqlite3 \
    rsync \
    less \
    && rm -rf /var/lib/apt/lists/* \
    # for cross compilation with raspberry pi 
    && rustup target add aarch64-unknown-linux-gnu \
    # create a non-root user to run everything in
    && useradd -m dev \
    # set default shell to bash for the new user
    && chsh -s /bin/bash dev \
    && rustup component add clippy rustfmt

RUN git clone https://github.com/ploiu/bashScripts /home/dev/bashScripts
USER dev

# better prompt and git status viewer
RUN bash -c "$(curl -fsSL https://raw.githubusercontent.com/ohmybash/oh-my-bash/master/tools/install.sh)"
RUN echo "for script in \$(find ~/bashScripts -name '*.sh'); do source \$script; done" >> /home/dev/.bashrc
