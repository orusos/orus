import QtQuick 2.0;
import calamares.slideshow 1.0;

Presentation
{
    id: presentation

    // The area around the slides is white without this.
    Rectangle {
        z: -1
        anchors.fill: parent
        color: "#191f24"
    }

    Slide {
        Rectangle {
            anchors.fill: parent
            color: "#191f24"
        }
        Text {
            anchors.centerIn: parent
            text: "Installing Orus OS"
            color: "#e6ebee"
            font.pixelSize: 28
        }
    }
}
