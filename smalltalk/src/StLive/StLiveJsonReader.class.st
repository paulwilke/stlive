"
Reads a request line. STON is a superset of JSON whose class tags (Point[1,2], ZnUrl{...}) build instances of image
classes. This reader refuses every class tag, so a request can only hold JSON values: maps, lists, strings, numbers,
true, false and null.
"
Class {
	#name : 'StLiveJsonReader',
	#superclass : 'STONReader',
	#package : 'StLive'
}

{ #category : 'private' }
StLiveJsonReader >> lookupClass: aName [
	^ (StLiveFailure
		code: 'bad_request'
		message: 'A request is JSON; the class tag ' , aName printString , ' is not accepted') signal
]
