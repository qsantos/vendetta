#include <stdio.h>
#include <SFML/Graphics.h>

int main(void)
{
	sfVideoMode mode = {800, 600, 32};
	sfRenderWindow* window = sfRenderWindow_create(mode, "SFML window", sfResize | sfClose, NULL);
	if (window == NULL)
	{
		fprintf(stderr, "Failed to open SFML window\n");
		return 1;
	}

	sfTexture* texture = sfTexture_createFromFile("character.png", NULL);
	if (texture == NULL)
	{
		fprintf(stderr, "Failed to load image\n");
		return 1;
	}

	sfSprite* sprite = sfSprite_create();
	sfSprite_setTexture(sprite, texture, sfTrue);

	sfIntRect rect = {0, 0, 24, 32};
	sfSprite_setTextureRect(sprite, rect);

	while (sfRenderWindow_isOpen(window))
	{
		sfEvent event;
		while (sfRenderWindow_pollEvent(window, &event))
		{
			if (event.type == sfEvtClosed)
				sfRenderWindow_close(window);
			else if (event.type == sfEvtKeyPressed && event.key.code == sfKeyEscape)
				sfRenderWindow_close(window);
		}

		sfRenderWindow_clear(window, sfBlack);
		sfRenderWindow_drawSprite(window, sprite, NULL);
		sfRenderWindow_display(window);
	}

	sfSprite_destroy(sprite);
	sfTexture_destroy(texture);
	sfRenderWindow_destroy(window);
	return 0;
}
