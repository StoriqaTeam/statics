# Statics server

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
