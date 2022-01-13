CREATE TABLE BlogEntries(
    id INT UNIQUE NOT NULL AUTO_INCREMENT,
    title TEXT NOT NULL,
    description TEXT NOT NULL,
    date DATE NOT NULL,
    path TEXT NOT NULL,
    PRIMARY KEY (id)
);
