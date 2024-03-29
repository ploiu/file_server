# will generate certs for you and place them here so you don't have to do so manually
echo "Generating public and private keys..."
openssl genrsa -out private-key.pem 3072
openssl rsa -in private-key.pem -pubout -out public-key.pem
echo "Created Keys"
openssl req -new -x509 -key private-key.pem -out cert.pem -config cert_options.conf
