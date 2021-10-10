## 1. Introduction

**SSFTP** is an acronym of **Super Simple File Transfer Protocol**. This is an implementation of a simple file transfer application over TCP using Rust programming language. The protocol is designed for client-server model.





## 2. Protocol Specification

**SSFTP** is an **application layer protocol** that allows file transfer from server to client, similar to the celebrated **FTP**, just simpler. It **does not allow** users to upload, delete, modify, rename the files and directories on the server side. Users only allowed to download and getting information about a file on the server side. It is also stateless since it does not need to remember any state.

### 2.1 Request

Take example by the HTTP, SSFTP make use of the idea of request methods, specifying the main purpose of the request. SSFTP only have three request methods: **GET**, **INFO**, **DIR**:

1. **GET**

   User sends a **GET** request when they want to download a file from the server.

2. **INFO**

   User sends a **INFO** request when they want to get only information of a file from the server.

3. **DIR**

   User sends a **DIR** request when they want to get the contents of a directory on server side.



#### Request format

###### Bytes representation of GET request

```
GET <path>\n
```

where `path` is the file path on server.



###### Bytes representation of INFO request

```
INFO <path>\n
```

where `path` is the file path on server.



###### Bytes representation of DIR request

```
DIR <path>\n
```

where `path` is the file path on server.



### 2.2 Response

Every response from server has the following format

```
<status-code>\n
<headers>\n
<payload>
```

#### 2.2.1 Status code

status code indicates the response type. The available status codes are:

1. `OK` - Everything is OK.
2. `NOT-EXIST` - Requested file or directory not exist.
3. `NOT-FILE` - `path` in a **GET** request is not a.
4. `NOT-DIRECTORY ` - `path` in a **DIR** request is a not a directory.
5. `SERVER-ERROR` - server error.
6. `BAD-REQUEST` - client provided a malformed request.



#### 2.2.2 Headers

Headers is a one-line json code, giving the information of the related to the response.



When the request is **GET** and status code is `OK`, the available headers are:

1. `content-length`: An integer, size of the `payload` in bytes.



When the request is **DIR** and status code is `OK`, the available headers are:

1. `content-length`: An integer, size of the `payload` in bytes.
2. `count`: An integer, number of files + directories in the requested directory.



When the request is **INFO** and status code is `OK`, the available headers are:

1. `type`: A string, either `'directory'` or `'file'`.

2. `content-length`: An integer, if `type` is `'file'`, size of the file in bytes.

   

#### 2.2.3 Payload

##### Payload of GET request

When the request is **GET** and status code is `OK`, the `payload` field contains full content of the file in raw bytes.



##### Payload of DIR request

When the request is **DIR** and status code is `OK`, the `payload` field contains many lines of entries, where each line is a JSON escaped file name or directory name. If it is a directory name, the entry ends with a '/'.

An example of `payload` field:

```
file-a\n
file-b\n
dir-c/\n
file-d
```



##### Payload of INFO request

Empty (0 bytes).



## 3. Examples

Suppose the directory `/home/myname/Pictures/` contains:

```
/home/myname/Pictures/:

baby.jpg
帅气的男银.jpg
cute-dogs/
	dog_a.jpg
	dog_b.jpg
	dog_c.jpg
```

That is, `/home/myname/Pictures` contains 2 files and 1 directory containing 3 files.

And the server listening and serve on the directory `/home/myname/Pictures/`.



If an incoming request is

```
GET /baby.jpg\n
```

Then the server return a response

```
OK\n
{"content-length": <size of baby.jpg>}\n
<content of baby.jpg>
```



If an incoming request is 

```
GET /girl.jpg\n
```

Then the server return a response

```
NOT-EXIST\n
{}\n
```



If an incoming request is 

```
DIR /\n
```

Then the server return a response

```
OK\n
{"content-length": 29, "count": 3}\n
baby.jpg\n
帅气的男银.jpg\n
cute-dogs/
```

