# Statics server

Statics is a microservice responsible for uploading different static assets like images, videos, etc.
The layered structure of the app is

`Application -> Controller -> Service -> HttpClient`

Currently available routes:

- `GET /healthcheck` - returns `"ok"` if the server is live
- `POST /images` - accepts multipart HTTP requests with `png` / `jpeg` images.
Returns `{"url": <url of uploaded image>}`. You can also use prefix with this url
to get different sizes: thumb - 40 pixels, small - 80 pixels, medium - 320 pixels,
large - 640 pixels. Example: `https://s3.amazonaws.com/storiqa-dev/img-2IpSsAjuxB8C.png` is original image,
`https://s3.amazonaws.com/storiqa-dev/img-2IpSsAjuxB8C-large.png` is large image.

## K8s deploy instructions

From project directory, issue the following commands

```
# Make sure you have those credentials before proceeding
kubectl create secret docker-registry stq \
  --docker-username="$registry_user" \
  --docker-password="$registry_pass" \
  --docker-server="$registry_host" \
  --docker-email="$registry_mail"

helm install --tls --name statics-pg -f docker/statics-pg_values.yaml stable/postgresql
kubectl create cm statics --from-file=config/

kubectl create -f docker/k8s/
```
