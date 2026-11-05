Class {
	#name : 'StLiveBaselineShapeTest',
	#superclass : 'StLiveTestCase',
	#package : 'StLive-Tests'
}

{ #category : 'groups' }
StLiveBaselineShapeTest >> testDefaultGroupLoadsOnlyTheFramework [
	| version |
	version := (Smalltalk at: #BaselineOfStLive) version.
	self assert: (commands packageNamesOfSpecNamed: 'default' in: version) asArray equals: #('StLive')
]

{ #category : 'groups' }
StLiveBaselineShapeTest >> testTestsGroupAddsTheTestPackage [
	| names |
	names := commands packageNamesOfSpecNamed: 'Tests' in: (Smalltalk at: #BaselineOfStLive) version.
	self assert: names asSet equals: #('StLive' 'StLive-Tests') asSet
]

{ #category : 'groups' }
StLiveBaselineShapeTest >> testTestPackageRequiresTheFramework [
	| spec |
	spec := (Smalltalk at: #BaselineOfStLive) version packageNamed: 'StLive-Tests'.
	self assert: (spec requires collect: [ :name | name asString ]) asArray equals: #('StLive')
]
