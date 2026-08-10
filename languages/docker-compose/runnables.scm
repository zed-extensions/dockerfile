(document (block_node (block_mapping (block_mapping_pair
    key: _ @_services
    value: (block_node (block_mapping (block_mapping_pair
            key: _ @service @run)))
    (#eq? @_services "services")
    (#set! tag docker-compose-service)))))
