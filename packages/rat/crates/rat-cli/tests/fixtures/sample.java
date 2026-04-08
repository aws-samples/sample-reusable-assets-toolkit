package com.example;

import java.util.Map;
import java.util.HashMap;

@SuppressWarnings("unchecked")
public class UserService {
    private final Map<String, String> users = new HashMap<>();

    public String getUser(String id) {
        return users.get(id);
    }

    public void createUser(String id, String name) {
        users.put(id, name);
    }
}

interface Repository {
    void save(String key, String value);
}

enum Status {
    ACTIVE,
    INACTIVE,
    PENDING
}
