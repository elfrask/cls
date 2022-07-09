extents KinematicBody2D

var karnic = false;
var sprite:Sprite = Sprite.new()

# Character
# "	"

var var_me = self

func _ready()
	main()
	add_to_group("obstacle")

	sprite.center = true
	self.add_child(sprite)

	pass

func destroy()
    
	self.queue_free()
	if True:
    	
        pass

	pass