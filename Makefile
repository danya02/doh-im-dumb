all: build deploy

build:
	docker buildx build --platform linux/arm64/v8,linux/amd64 -f Dockerfile . --tag registry.danya02.ru/danya02/doh-im-dumb:latest --builder kube --push

deploy:
	kubectl apply -f deploy.yaml

delete:
	kubectl delete -f deploy.yaml


initialize_builder:
	docker buildx create --bootstrap --name=local --driver=docker-container --platform=linux/arm64/v8,linux/amd64

delete_builder:
	docker buildx rm local