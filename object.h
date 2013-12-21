#ifndef OBJECT_H
#define OBJECT_H

typedef enum
{
	O_CHARACTER,
	O_MINE,
} otype_t;

typedef struct object object_t;

struct object
{
	otype_t t;

	float x;
	float y;
	float w;
	float h;
};

#endif
