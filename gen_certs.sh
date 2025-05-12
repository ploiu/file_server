# will generate certs for you and place them here so you don't have to do so manually
echo "generating private root ca key"
openssl genrsa -out ca.key 3072
echo "generating root ca cert"
openssl req -x509 -new -nodes -key ca.key -sha256 -days 30000 -out ca.crt -config ./cert_options.conf
echo "Generating private key"
openssl genrsa -out private-key.pem 3072
echo "generating server signing request"
openssl req -new -key private-key.pem -out server.csr -config cert_options.conf
echo "signing server csr with ca certificate"
openssl x509 -req -in server.csr -CA ca.crt -CAkey ca.key -CAcreateserial -out cert.pem -days 30000 -sha256 -extfile cert_options.conf -extensions v3_req
openssl rsa -in private-key.pem -pubout -out public-key.pem
echo "combining server cert and key for mobile devices"
openssl pkcs12 -export -out file_server.p12 -inkey private-key.pem -in cert.pem -certfile ca.crt
